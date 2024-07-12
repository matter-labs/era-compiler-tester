//!
//! The EVM deploy contract call input variant.
//!

use std::collections::HashMap;
use std::hash::RandomState;
use std::io::Read;
use std::str::FromStr;
use std::sync::Arc;
use std::sync::Mutex;

use revm::db::states::plain_account::PlainStorage;

use revm::primitives::Bytes;
use revm::primitives::EVMError;
use revm::primitives::ExecutionResult;
use revm::primitives::TxKind;
use revm::primitives::B256;
use revm::primitives::KECCAK_EMPTY;
use revm::primitives::U256;
use revm::Database;
use revm::Evm;
use revm::State;
use solidity_adapter::EVMVersion;
use web3::ethabi::Hash;
use zkevm_opcode_defs::p256::U256 as ZKU256;

use crate::compilers::mode::Mode;
use crate::summary::Summary;
use crate::test::case::input::calldata::Calldata;
use crate::test::case::input::output::Output;
use crate::test::case::input::storage::Storage;
use crate::vm::eravm::deployers::EraVMDeployer;
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
    pub fn run_revm<'a, EXT, DB: Database>(
        self,
        summary: Arc<Mutex<Summary>>,
        vm: revm::Evm<'a, EXT, State<DB>>,
        mode: Mode,
        test_group: Option<String>,
        name_prefix: String,
        evm_builds: &HashMap<String, Build, RandomState>,
    ) -> revm::Evm<'a, EXT,State<DB>> {
        let name = format!("{}[#deployer:{}]", name_prefix, self.identifier);

        //vm.populate_storage(self.storage.inner);
        let build = evm_builds
            .get(self.identifier.as_str())
            .expect("Always valid");
        let mut deploy_code = build.deploy_build.bytecode.to_owned();
        deploy_code.extend(self.calldata.inner.clone());

        let mut new_vm: Evm<EXT, State<DB>> = vm.modify().modify_env(|env| {
            env.tx.caller = web3_address_to_revm_address(&self.caller);
            env.tx.data = revm::primitives::Bytes::from(deploy_code);
            env.tx.value = revm::primitives::U256::from(self.value.unwrap_or_default());
            env.tx.transact_to = TxKind::Create;
        }).build();

        let res = match new_vm.transact_commit() {
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
                return new_vm;
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
        }

        new_vm
    }

    pub fn add_balance(&self, cache: &mut revm::CacheState) {
        let rich_addresses: Vec<web3::types::Address> = SystemContext::get_rich_addresses();
        let acc_info = if rich_addresses.contains(&self.caller) {
            let address_bytes: &mut [u8; 32] = &mut [0; 32];
            web3::types::U256::from(self.caller.as_bytes()).to_big_endian(address_bytes);
            revm::primitives::AccountInfo {
                balance: (U256::from(1) << 100) + U256::from_str("63615000000000").unwrap(),
                code_hash: KECCAK_EMPTY,
                code: None,
                nonce: 1,
            }
        } else {
            let address_bytes: &mut [u8; 32] = &mut [0; 32];
            web3::types::U256::from(self.caller.as_bytes()).to_big_endian(address_bytes);
            revm::primitives::AccountInfo {
                balance: U256::ZERO,
                code_hash: KECCAK_EMPTY,
                code: None,
                nonce: 1,
            }
        };
        cache.insert_account_with_storage(
            web3_address_to_revm_address(&self.caller),
            acc_info,
            PlainStorage::default(),
        );

        // web3::types::U256::from(self.caller.as_bytes()).to_big_endian(caller_bytes);
        // let acc_info = revm::primitives::AccountInfo {
        //     balance: U256::from(1) << 100,c
        //     code_hash: KECCAK_EMPTY,
        //     code: None,
        //     nonce: 1,
        // };
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
