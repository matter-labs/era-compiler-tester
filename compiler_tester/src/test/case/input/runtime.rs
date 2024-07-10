//!
//! The contract call input variant.
//!

use std::collections::HashMap;
use std::convert::Infallible;
use std::hash::RandomState;
use std::sync::Arc;
use std::sync::Mutex;

use revm::db::states::plain_account::PlainStorage;
use revm::db::EmptyDBTyped;
use revm::db::State;
use revm::primitives::Account;
use revm::primitives::AccountStatus;
use revm::primitives::Address;
use revm::primitives::Bytes;
use revm::primitives::Env;
use revm::primitives::ExecutionResult;
use revm::primitives::TxKind;
use revm::primitives::KECCAK_EMPTY;
use revm::primitives::U256;
use revm::Database;
use revm::DatabaseCommit;
use revm::Evm;

use crate::compilers::mode::Mode;
use crate::summary::Summary;
use crate::test::case::input::calldata::Calldata;
use crate::test::case::input::output::Output;
use crate::test::case::input::storage::Storage;
use crate::vm::eravm::EraVM;
use crate::vm::evm::input::build::Build;
use crate::vm::evm::EVM;

use super::output::event::Event;

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
    ) -> revm::Evm<'a, EXT,State<DB>> {
        let name = format!("{}[{}:{}]", name_prefix, self.name, index);

        let mut vm: Evm<EXT, State<DB>> = vm.modify().modify_env(|env| {
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
            web3::types::U256::from(self.address.as_bytes()).to_big_endian(address_bytes);
            env.tx.transact_to = TxKind::Call(Address::from_word(revm::primitives::FixedBytes::new(*address_bytes)));
       }).build();

       let res = match vm.transact_commit() {
           Ok(res) => res,
           Err(error) => {
               Summary::invalid(summary, Some(mode), name, "error on commit");
               return vm;
           }
       };

        //vm.populate_storage(self.storage.inner)
        
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
                    if data.len() < 32 {
                        let mut value = [0u8; 32];
                        value[..data.len()].copy_from_slice(data);
                        return_data_value.push(super::value::Value::Certain(web3::types::U256::from_big_endian(&value)));
                    } else {
                        let mut value = [0u8; 32];
                        value.copy_from_slice(data);
                        return_data_value.push(super::value::Value::Certain(web3::types::U256::from_big_endian(&value)));
                    }
                }
                let events = logs.into_iter().map(|log| {
                    let mut topics = vec![];
                    for topic in log.data.topics().iter() {
                        let mut topic_value = [0u8; 32];
                        topic_value.copy_from_slice(topic.as_slice());
                        topics.push(super::value::Value::Certain(web3::types::U256::from_big_endian(&topic_value)));
                    }
                    let mut data = vec![];
                    data.extend_from_slice(&log.data.data);
                    let mut data_value = vec![];
                    for data in data.chunks(32) {
                        if data.len() < 32 {
                            let mut value = [0u8; 32];
                            value[..data.len()].copy_from_slice(data);
                            data_value.push(super::value::Value::Certain(web3::types::U256::from_big_endian(&value)));
                        } else {
                            let mut value = [0u8; 32];
                            value.copy_from_slice(data);
                            data_value.push(super::value::Value::Certain(web3::types::U256::from_big_endian(&value)));
                        }
                    }
                    Event::new(Some(web3::types::Address::from_slice(&log.address.as_slice())), topics, data_value)
                }).collect();
                let output = Output::new(return_data_value, false, events);
                output
            }
            ExecutionResult::Revert{gas_used, output} => {
                let mut return_data = vec![];
                return_data.extend_from_slice(&output);
                let mut return_data_value = vec![];
                for data in return_data.chunks(32) {
                    if data.len() < 32 {
                        let mut value = [0u8; 32];
                        value[..data.len()].copy_from_slice(data);
                        return_data_value.push(super::value::Value::Certain(web3::types::U256::from_big_endian(&value)));
                    } else {
                        let mut value = [0u8; 32];
                        value.copy_from_slice(data);
                        return_data_value.push(super::value::Value::Certain(web3::types::U256::from_big_endian(&value)));
                    }
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
        let caller_bytes: &mut [u8; 32] = &mut [0; 32];
        web3::types::U256::from(self.caller.as_bytes()).to_big_endian(caller_bytes);
        let acc_info = revm::primitives::AccountInfo {
            balance: U256::MAX,
            code_hash: KECCAK_EMPTY,
            code: None,
            nonce: 1,
        };

        cache.insert_account_with_storage(Address::from_word(revm::primitives::FixedBytes::new(*caller_bytes)), acc_info, PlainStorage::default());
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
