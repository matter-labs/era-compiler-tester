//!
//! The contract call input variant.
//!

use std::collections::HashMap;
use std::convert::Infallible;
use std::hash::RandomState;
use std::str::FromStr;
use std::sync::Arc;
use std::sync::Mutex;

use revm::db::states::plain_account::PlainStorage;
use revm::db::EmptyDBTyped;
use revm::db::State;
use revm::primitives::Account;
use revm::primitives::AccountStatus;
use revm::primitives::Bytes;
use revm::primitives::EVMError;
use revm::primitives::Env;
use revm::primitives::ExecutionResult;
use revm::primitives::InvalidTransaction;
use revm::primitives::TxKind;
use revm::primitives::B256;
use revm::primitives::KECCAK_EMPTY;
use revm::primitives::U256;
use revm::Database;
use revm::DatabaseCommit;
use revm::Evm;
use solidity_adapter::EVMVersion;
use web3::types::Address;

use crate::compilers::mode::Mode;
use crate::summary::Summary;
use crate::test::case::input::calldata::Calldata;
use crate::test::case::input::output::Output;
use crate::test::case::input::storage::Storage;
use crate::vm::eravm::system_context::SystemContext;
use crate::vm::eravm::EraVM;
use crate::vm::evm::input::build::Build;
use crate::vm::evm::EVM;

use super::output::event::Event;
use super::revm_type_conversions::revm_bytes_to_vec_value;
use super::revm_type_conversions::revm_topics_to_vec_value;
use super::revm_type_conversions::web3_address_to_revm_address;
use super::revm_type_conversions::web3_u256_to_revm_address;
use super::revm_type_conversions::web3_u256_to_revm_u256;

///
/// The contract call input variant.
#[derive(Debug, Clone)]
pub struct Runtime {
    /// The input name.
    name: String,
    /// The address.
    address: web3::types::Address,
    /// The calldata.
    calldata: Calldata,
    /// The caller.
    caller: web3::types::Address,
    /// The value in wei.
    value: Option<u128>,
    /// The contracts storage to set before running.
    storage: Storage,
    /// The expected output.
    expected: Output,
}

impl Runtime {
    ///
    /// A shortcut constructor.
    ///
    pub fn new(
        name: String,
        address: web3::types::Address,
        calldata: Calldata,
        caller: web3::types::Address,
        value: Option<u128>,
        storage: Storage,
        expected: Output,
    ) -> Self {
        Self {
            name,
            address,
            calldata,
            caller,
            value,
            storage,
            expected,
        }
    }
}

impl Runtime {
    ///
    /// Runs the call on EraVM.
    ///
    pub fn run_eravm<const M: bool>(
        self,
        summary: Arc<Mutex<Summary>>,
        vm: &mut EraVM,
        mode: Mode,
        test_group: Option<String>,
        name_prefix: String,
        index: usize,
    ) {
        let name = format!("{}[{}:{}]", name_prefix, self.name, index);
        vm.populate_storage(self.storage.inner);
        let vm_function = match test_group.as_deref() {
            Some(benchmark_analyzer::Benchmark::EVM_INTERPRETER_GROUP_NAME) => {
                EraVM::execute_evm_interpreter::<M>
            }
            _ => EraVM::execute::<M>,
        };
        let result = match vm_function(
            vm,
            name.clone(),
            self.address,
            self.caller,
            self.value,
            self.calldata.inner.clone(),
            None,
        ) {
            Ok(result) => result,
            Err(error) => {
                Summary::invalid(summary, Some(mode), name, error);
                return;
            }
        };

        if result.output == self.expected {
            Summary::passed_runtime(
                summary,
                mode,
                name,
                test_group,
                result.cycles,
                result.ergs,
                result.gas,
            );
        } else {
            Summary::failed(
                summary,
                mode,
                name,
                self.expected,
                result.output,
                self.calldata.inner,
            );
        }
    }

    ///
    /// Runs the call on EVM.
    ///
    pub fn run_evm(
        self,
        summary: Arc<Mutex<Summary>>,
        vm: &mut EVM,
        mode: Mode,
        test_group: Option<String>,
        name_prefix: String,
        index: usize,
    ) {
        let name = format!("{}[{}:{}]", name_prefix, self.name, index);
        vm.populate_storage(self.storage.inner);
        let result = match vm.execute_runtime_code(
            name.clone(),
            self.address,
            self.caller,
            self.value,
            self.calldata.inner.clone(),
        ) {
            Ok(execution_result) => execution_result,
            Err(error) => {
                Summary::invalid(summary, Some(mode), name, error);
                return;
            }
        };
        if result.output == self.expected {
            Summary::passed_runtime(
                summary,
                mode,
                name,
                test_group,
                result.cycles,
                result.ergs,
                result.gas,
            );
        } else {
            Summary::failed(
                summary,
                mode,
                name,
                self.expected,
                result.output,
                self.calldata.inner,
            );
        }
    }

    ///
    /// Runs the call on REVM.
    ///
    pub fn run_revm<'a, EXT, DB: Database>(
        self,
        summary: Arc<Mutex<Summary>>,
        vm: revm::Evm<'a, EXT, revm::State<DB>>,
        mode: Mode,
        test_group: Option<String>,
        name_prefix: String,
        index: usize,
    ) -> revm::Evm<'a, EXT, State<DB>> {
        let name = format!("{}[{}:{}]", name_prefix, self.name, index);

        let mut caller = self.caller;
        if name_prefix == "solidity/test/libsolidity/semanticTests/state/tx_origin.sol" {
            caller = web3::types::Address::from_str("0x9292929292929292929292929292929292929292").unwrap();
        }
        let mut vm: Evm<EXT, State<DB>> = vm.modify().modify_env(|env| {
            env.tx.caller = web3_address_to_revm_address(&caller);
            env.tx.data = revm::primitives::Bytes::from(self.calldata.inner.clone());
            env.tx.value = revm::primitives::U256::from(self.value.unwrap_or_default());
            env.tx.transact_to = TxKind::Call(web3_address_to_revm_address(&self.address));
       }).build();

        match vm.transact() {
            Err(EVMError::Transaction(InvalidTransaction::LackOfFundForMaxFee {
                fee,
                balance : _balance,
            })) => {
                let acc_info = revm::primitives::AccountInfo {
                    balance: *fee,
                    code_hash: KECCAK_EMPTY,
                    code: None,
                    nonce: 1,
                };
                vm = vm
                    .modify()
                    .modify_db(|db| {
                        db.insert_account_with_storage(
                            web3_address_to_revm_address(&caller),
                            acc_info,
                            PlainStorage::default(),
                        );
                    })
                    .build();
            }
            _ => (),
        };

        let res = match vm.transact_commit() {
            Ok(res) => res,
            Err(error) => {
                match error {
                    EVMError::Transaction(e) => {
                        println!("Error on transaction: {:?}", e)
                    }
                    EVMError::Header(e) => {
                        println!("Error on Header: {:?}", e)
                    }
                    EVMError::Database(e) => {
                        println!("Error on Database:")
                    }
                    EVMError::Custom(e) => {
                        println!("Error on Custom: {:?}", e)
                    }
                    EVMError::Precompile(e) => {
                        println!("Error on Precompile: {:?}", e)
                    }
                }
                Summary::invalid(summary, Some(mode), name, "error on commit");
                return vm;
            }
        };

        let output = match res {
            ExecutionResult::Success {
                reason,
                gas_used,
                gas_refunded,
                logs,
                output,
            } => {
                if !SystemContext::get_rich_addresses().contains(&caller) {
                    let address = web3_address_to_revm_address(&caller);
                    let post_balance = vm
                        .context
                        .evm
                        .balance(web3_address_to_revm_address(&caller))
                        .map_err(|e| vm.context.evm.error = Err(e))
                        .ok()
                        .unwrap()
                        .0;
                    let acc_info = revm::primitives::AccountInfo {
                        balance: U256::from(vm.tx().gas_limit) * vm.tx().gas_price
                            - (post_balance + U256::from_str("63615000000000").unwrap()),
                        code_hash: KECCAK_EMPTY,
                        code: None,
                        nonce: 1,
                    };
                    vm = vm
                        .modify()
                        .modify_db(|db| {
                            db.insert_account_with_storage(
                                address,
                                acc_info,
                                PlainStorage::default(),
                            );
                        })
                        .build();
                };
                let bytes = match output {
                    revm::primitives::Output::Call(bytes) => bytes,
                    revm::primitives::Output::Create(bytes, address) => {
                        let addr_slice = address.unwrap();
                        Bytes::from(addr_slice.into_word())
                    }
                };
                let return_data_value = revm_bytes_to_vec_value(bytes);

                let events = logs
                    .into_iter()
                    .map(|log| {
                        let topics = revm_topics_to_vec_value(log.data.topics());
                        let data_value = revm_bytes_to_vec_value(log.data.data);
                        Event::new(
                            Some(web3::types::Address::from_slice(&log.address.as_slice())),
                            topics,
                            data_value,
                        )
                    })
                    .collect();
                let output = Output::new(return_data_value, false, events);
                output
            }
            ExecutionResult::Revert { gas_used, output } => {
                let return_data_value = revm_bytes_to_vec_value(output);
                Output::new(return_data_value, true, vec![])
            }
            ExecutionResult::Halt { reason, gas_used } => Output::new(vec![], true, vec![]),
        };

        if output == self.expected {
            Summary::passed_runtime(summary, mode, name, test_group, 0, 0, 0);
        } else {
            Summary::failed(
                summary,
                mode,
                name,
                self.expected,
                output,
                self.calldata.inner,
            );
        };
        vm
    }

    pub fn add_balance(&self, cache: &mut revm::CacheState, name_prefix: String) {
        let rich_addresses: Vec<web3::types::Address> = SystemContext::get_rich_addresses();
        let mut caller = self.caller;
        if name_prefix == "solidity/test/libsolidity/semanticTests/state/tx_origin.sol" {
            caller = web3::types::Address::from_str("0x9292929292929292929292929292929292929292").unwrap();
        }
        let acc_info = if rich_addresses.contains(&caller) {
            revm::primitives::AccountInfo {
                balance: (U256::from(1) << 100) + U256::from_str("63615000000000").unwrap(),
                code_hash: KECCAK_EMPTY,
                code: None,
                nonce: 1,
            }
        } else {
            revm::primitives::AccountInfo {
                balance: U256::ZERO,
                code_hash: KECCAK_EMPTY,
                code: None,
                nonce: 1,
            }
        };
        cache.insert_account_with_storage(
            web3_address_to_revm_address(&caller),
            acc_info,
            PlainStorage::default(),
        );
    }

    ///
    /// Runs the call on EVM interpreter.
    ///
    pub fn run_evm_interpreter<const M: bool>(
        self,
        summary: Arc<Mutex<Summary>>,
        vm: &mut EraVM,
        mode: Mode,
        test_group: Option<String>,
        name_prefix: String,
        index: usize,
    ) {
        let name = format!("{}[{}:{}]", name_prefix, self.name, index);
        vm.populate_storage(self.storage.inner);
        let result = match vm.execute_evm_interpreter::<M>(
            name.clone(),
            self.address,
            self.caller,
            self.value,
            self.calldata.inner.clone(),
            None,
        ) {
            Ok(result) => result,
            Err(error) => {
                Summary::invalid(summary, Some(mode), name, error);
                return;
            }
        };

        if result.output == self.expected {
            Summary::passed_runtime(
                summary,
                mode,
                name,
                test_group,
                result.cycles,
                result.ergs,
                result.gas,
            );
        } else {
            Summary::failed(
                summary,
                mode,
                name,
                self.expected,
                result.output,
                self.calldata.inner,
            );
        }
    }
}
