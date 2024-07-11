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
use revm::primitives::Address;
use revm::primitives::Bytes;
use revm::primitives::EVMError;
use revm::primitives::Env;
use revm::primitives::ExecutionResult;
use revm::primitives::TxKind;
use revm::primitives::KECCAK_EMPTY;
use revm::primitives::U256;
use revm::Database;
use revm::DatabaseCommit;
use revm::Evm;
use solidity_adapter::EVMVersion;

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
    pub fn run_revm<'a, EXT,DB: Database>(
        self,
        summary: Arc<Mutex<Summary>>,
        vm: revm::Evm<'a, EXT,revm::State<DB>>,
        mode: Mode,
        test_group: Option<String>,
        name_prefix: String,
        index: usize,
        evm_version: Option<EVMVersion>,
    ) -> revm::Evm<'a, EXT,State<DB>> {
        let name = format!("{}[{}:{}]", name_prefix, self.name, index);

        let mut vm: Evm<EXT, State<DB>> = vm.modify().modify_env(|env| {
            let evm_context = SystemContext::get_constants_evm(evm_version);
            env.cfg.chain_id = evm_context.chain_id;
            env.block.number = U256::from(evm_context.block_number);
            let coinbase = web3::types::U256::from_str_radix(evm_context.coinbase,16).unwrap();
            env.block.coinbase = web3_u256_to_revm_address(coinbase);
            env.block.timestamp = U256::from(evm_context.block_timestamp);
            //env.block.gas_limit = U256::from(evm_context.block_gas_limit);
            env.block.basefee = U256::from(evm_context.base_fee);
            let block_difficulty = web3::types::U256::from_str_radix(evm_context.block_difficulty,16).unwrap();
            env.block.difficulty = web3_u256_to_revm_u256(block_difficulty);
            //env.block.prevrandao = ;
            env.tx.caller = web3_address_to_revm_address(self.caller);
            env.tx.gas_price = U256::from(0xb2d05e00_u32);
            //env.tx.gas_priority_fee = ;
            //env.tx.blob_hashes = ;
            //env.tx.max_fee_per_blob_gas = 
            env.tx.gas_limit = 0xffffffff;
            env.tx.data = revm::primitives::Bytes::from(self.calldata.inner.clone());
            env.tx.value = revm::primitives::U256::from(self.value.unwrap_or_default());
            env.tx.access_list = vec![];
            env.tx.transact_to = TxKind::Call(web3_address_to_revm_address(self.address));
       }).build();

        //vm.populate_storage(self.storage.inner)
        
        let res = match vm.transact_commit() {
            Ok(res) => res,
            Err(error) => {
                match error {
                    EVMError::Transaction(e) => {
                        println!("Error on transaction: {:?}", e)
                    },
                    EVMError::Header(e) => {
                        println!("Error on Header: {:?}", e)
                    },
                    EVMError::Database(e) => {
                        println!("Error on Database:")
                    },
                    EVMError::Custom(e) => {
                        println!("Error on Custom: {:?}", e)
                    },
                    EVMError::Precompile(e) => {
                        println!("Error on Precompile: {:?}", e)
                    },
                }
                Summary::invalid(summary, Some(mode), name, "error on commit");
                return vm;
            }
        };
        
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
                let return_data_value = revm_bytes_to_vec_value(bytes);

                let events = logs.into_iter().map(|log| {
                    let topics = revm_topics_to_vec_value(log.data.topics());
                    let data_value = revm_bytes_to_vec_value(log.data.data);
                    Event::new(Some(web3::types::Address::from_slice(&log.address.as_slice())), topics, data_value)
                }).collect();
                let output = Output::new(return_data_value, false, events);
                output
            }
            ExecutionResult::Revert{gas_used, output} => {
                let return_data_value = revm_bytes_to_vec_value(output);
                Output::new(return_data_value, true, vec![])
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
        };
        vm
    }

    pub fn add_balance(&self, cache: &mut revm::CacheState) {
        let acc_info = revm::primitives::AccountInfo {
            balance: U256::MAX,
            code_hash: KECCAK_EMPTY,
            code: None,
            nonce: 1,
        };

        cache.insert_account_with_storage(web3_address_to_revm_address(self.caller), acc_info, PlainStorage::default());
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
