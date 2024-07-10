//!
//! The contract call input variant.
//!

use std::collections::HashMap;
use std::hash::RandomState;
use std::sync::Arc;
use std::sync::Mutex;

use revm::db::states::plain_account::PlainStorage;
use revm::primitives::Address;
use revm::primitives::Bytes;
use revm::primitives::Env;
use revm::primitives::ExecutionResult;
use revm::primitives::TxKind;
use revm::primitives::KECCAK_EMPTY;
use revm::primitives::U256;
use revm::Evm;

use crate::compilers::mode::Mode;
use crate::summary::Summary;
use crate::test::case::input::calldata::Calldata;
use crate::test::case::input::output::Output;
use crate::test::case::input::storage::Storage;
use crate::vm::eravm::EraVM;
use crate::vm::evm::input::build::Build;
use crate::vm::evm::EVM;

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
    pub fn run_revm<EXT, DB: revm::db::Database>(
        self,
        summary: Arc<Mutex<Summary>>,
        vm: &mut revm::Evm<EXT, DB>,
        mode: Mode,
        test_group: Option<String>,
        name_prefix: String,
        index: usize,
    ) {
        let name = format!("{}[{}:{}]", name_prefix, self.name, index);

        //vm.populate_storage(self.storage.inner)

        let mut env = Box::<Env>::default();


        // env.cfg.chain_id = ;
        //env.block.number = ;
        //env.block.coinbase = ;
        //env.block.timestamp = ;
        //env.block.gas_limit = U256::from(0xffffffff_u32);
        //env.block.basefee = ;
        //env.block.difficulty = ;
        //env.block.prevrandao = ;

        let caller_bytes: &mut [u8; 32] = &mut [0; 32];
        web3::types::U256::from(self.caller.as_bytes()).to_big_endian(caller_bytes);
        env.tx.caller = Address::from_word(revm::primitives::FixedBytes::new(*caller_bytes));
        env.tx.gas_price = U256::from(0xb2d05e00_u32);
        //env.tx.gas_priority_fee = ;
        //env.tx.blob_hashes = ;
        //env.tx.max_fee_per_blob_gas = 
        env.tx.gas_limit = 0xffffffff;
        env.tx.data = revm::primitives::Bytes::from(self.calldata.inner.clone());
        env.tx.value = revm::primitives::U256::from(self.value.unwrap_or_default());
        env.tx.access_list = vec![];
        let address_bytes: &mut [u8; 32] = &mut [0; 32];
        web3::types::U256::from(self.caller.as_bytes()).to_big_endian(address_bytes);
        env.tx.transact_to = TxKind::Call(Address::from_word(revm::primitives::FixedBytes::new(*address_bytes)));

        let mut cache_state = revm::CacheState::new(false);

        let acc_info = revm::primitives::AccountInfo {
            balance: U256::MAX,
            code_hash: KECCAK_EMPTY,
            code: None,
            nonce: 1,
        };
        cache_state.insert_account_with_storage(env.tx.caller, acc_info, PlainStorage::default());


        let mut state = revm::db::State::builder().with_cached_prestate(cache_state)
                    .build();
                let mut evm = Evm::builder()
                    .with_db(&mut state)
                    .modify_env(|e| e.clone_from(&env))
                    .build();

        let res = evm.transact_commit().expect("Execution Error");
        
        let output = match res {
            ExecutionResult::Success{reason, gas_used, gas_refunded, logs, output} => {
                let bytes = match output {
                    revm::primitives::Output::Call(bytes) => {
                        bytes
                    }
                    revm::primitives::Output::Create(bytes,address) => {
                        let addr_slice = address.unwrap();
                        Bytes::from(addr_slice.into_word())
                    }
                };
                let mut return_data = vec![];
                return_data.extend_from_slice(&bytes);
                let mut return_data_value = vec![];
                for data in return_data.chunks(32) {
                    let mut value = [0u8; 32];
                    value.copy_from_slice(data);
                    return_data_value.push(super::value::Value::Certain(web3::types::U256::from_big_endian(&value)));
                }
                let output = Output::new(return_data_value, false, vec![]);
                output
            }
            ExecutionResult::Revert{gas_used, output} => {
                let mut return_data = vec![];
                return_data.extend_from_slice(&output);
                let mut return_data_value = vec![];
                for data in return_data.chunks(32) {
                    let mut value = [0u8; 32];
                    value.copy_from_slice(data);
                    return_data_value.push(super::value::Value::Certain(web3::types::U256::from_big_endian(&value)));
                }
                let output = Output::new(return_data_value, true, vec![]);
                output
            }
            ExecutionResult::Halt{reason, gas_used} => {
                Output::new(vec![], true, vec![])
            }
        };

        if output == self.expected {
            Summary::passed_runtime(
                summary,
                mode,
                name,
                test_group,
                0,
                0,
                0,
            );
        } else {
            Summary::failed(
                summary,
                mode,
                name,
                self.expected,
                output,
                self.calldata.inner,
            );
        }
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
