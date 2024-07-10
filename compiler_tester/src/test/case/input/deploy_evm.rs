//!
//! The EVM deploy contract call input variant.
//!

use std::collections::HashMap;
use std::convert::Infallible;
use std::hash::RandomState;
use std::io::Read;
use std::sync::Arc;
use std::sync::Mutex;

use revm::db::states::plain_account::PlainStorage;
use revm::db::EmptyDBTyped;
use revm::primitives::bitvec::view::BitViewSized;
use revm::primitives::hex::FromHex;
use revm::primitives::Account;
use revm::primitives::AccountStatus;
use revm::primitives::Address;
use revm::primitives::Bytes;
use revm::primitives::EVMError;
use revm::primitives::Env;
use revm::primitives::ExecutionResult;
use revm::primitives::LogData;
use revm::primitives::TxKind;
use revm::primitives::KECCAK_EMPTY;
use revm::primitives::U256;
use revm::Database;
use revm::DatabaseCommit;
use revm::Evm;
use revm::State;
use web3::ethabi::Hash;
use zkevm_opcode_defs::p256::U256 as ZKU256;

use crate::compilers::mode::Mode;
use crate::summary::Summary;
use crate::test::case::input::calldata::Calldata;
use crate::test::case::input::output::Output;
use crate::test::case::input::storage::Storage;
use crate::vm::eravm::deployers::EraVMDeployer;
use crate::vm::eravm::EraVM;
use crate::vm::evm::input::build::Build;
use crate::vm::evm::EVM;

use super::output::event::Event;

///
/// The EVM deploy contract call input variant.
///
#[derive(Debug, Clone)]
pub struct DeployEVM {
    /// The contract identifier.
    identifier: String,
    /// The contract init code.
    init_code: Vec<u8>,
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

impl DeployEVM {
    ///
    /// A shortcut constructor.
    ///
    pub fn new(
        identifier: String,
        init_code: Vec<u8>,
        calldata: Calldata,
        caller: web3::types::Address,
        value: Option<u128>,
        storage: Storage,
        expected: Output,
    ) -> Self {
        Self {
            identifier,
            init_code,
            calldata,
            caller,
            value,
            storage,
            expected,
        }
    }
}

impl DeployEVM {
    ///
    /// Runs the deploy transaction on native EVM.
    ///
    pub fn run_evm(
        self,
        summary: Arc<Mutex<Summary>>,
        vm: &mut EVM,
        mode: Mode,
        test_group: Option<String>,
        name_prefix: String,
    ) {
        let name = format!("{}[#deployer:{}]", name_prefix, self.identifier);

        vm.populate_storage(self.storage.inner);
        let result = match vm.execute_deploy_code(
            name.clone(),
            self.identifier.as_str(),
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
                0,
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
    /// Runs the deploy transaction on native REVM.
    ///
    pub fn run_revm<'a, EXT,DB: Database + DatabaseCommit>(
        self,
        summary: Arc<Mutex<Summary>>,
        vm: revm::Evm<'a, EXT,DB>,
        mode: Mode,
        test_group: Option<String>,
        name_prefix: String,
        evm_builds: &HashMap<String, Build, RandomState>,
    ) -> revm::Evm<'a, EXT,DB> {
        let name = format!("{}[#deployer:{}]", name_prefix, self.identifier);

        //vm.populate_storage(self.storage.inner);
        let build = evm_builds.get(self.identifier.as_str()).expect("Always valid");
        let mut deploy_code = build.deploy_build.bytecode.to_owned();
        deploy_code.extend(self.calldata.inner.clone());

        let mut new_vm: Evm<EXT, DB> = vm.modify().modify_env(|env| {
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
            env.tx.data = revm::primitives::Bytes::from(deploy_code);
            env.tx.value = revm::primitives::U256::from(self.value.unwrap_or_default());
            env.tx.access_list = vec![];
            env.tx.transact_to = TxKind::Create;
        }).modify_db(|db| {
            let acc_info = revm::primitives::AccountInfo {
                balance: U256::MAX,
                code_hash: KECCAK_EMPTY,
                code: None,
                nonce: 1,
            };
            let mut changes = HashMap::new();
            let caller_bytes: &mut [u8; 32] = &mut [0; 32];
            web3::types::U256::from(self.caller.as_bytes()).to_big_endian(caller_bytes);
            let account = Account{info: acc_info, storage: HashMap::new(), status: AccountStatus::Created};
            changes.insert(Address::from_word(revm::primitives::FixedBytes::new(*caller_bytes)), account);
            db.commit(changes);
        }).build();
        let caller_bytes: &mut [u8; 32] = &mut [0; 32];
        web3::types::U256::from(self.caller.as_bytes()).to_big_endian(caller_bytes);

        let res = match new_vm.transact_commit() {
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
                return new_vm;
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
                let mut return_data = vec![];
                return_data.extend_from_slice(&bytes);
                let mut return_data_value = vec![];
                for data in return_data.chunks(32) {
                    let mut value = [0u8; 32];
                    value.copy_from_slice(data);
                    return_data_value.push(super::value::Value::Certain(web3::types::U256::from_big_endian(&value)));
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

        new_vm
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
    /// Runs the deploy transaction on EVM interpreter.
    ///
    pub fn run_evm_interpreter<D, const M: bool>(
        self,
        summary: Arc<Mutex<Summary>>,
        vm: &mut EraVM,
        mode: Mode,
        deployer: &mut D,
        test_group: Option<String>,
        name_prefix: String,
    ) where
        D: EraVMDeployer,
    {
        let name = format!("{}[#deployer:{}]", name_prefix, self.identifier);

        let size = self.init_code.len();

        vm.populate_storage(self.storage.inner);
        let result = match deployer.deploy_evm::<M>(
            name.clone(),
            self.caller,
            self.init_code,
            self.calldata.inner.clone(),
            self.value,
            vm,
        ) {
            Ok(result) => result,
            Err(error) => {
                Summary::invalid(summary, Some(mode), name, error);
                return;
            }
        };
        if result.output == self.expected {
            Summary::passed_deploy(
                summary,
                mode,
                name,
                test_group,
                size,
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
