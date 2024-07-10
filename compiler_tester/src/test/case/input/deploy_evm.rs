//!
//! The EVM deploy contract call input variant.
//!

use std::collections::HashMap;
use std::hash::RandomState;
use std::sync::Arc;
use std::sync::Mutex;

use revm::db::states::plain_account::PlainStorage;
use revm::primitives::hex::FromHex;
use revm::primitives::Address;
use revm::primitives::Bytes;
use revm::primitives::Env;
use revm::primitives::ExecutionResult;
use revm::primitives::KECCAK_EMPTY;
use revm::primitives::U256;
use revm::Evm;
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
    pub fn run_revm<EXT,DB: revm::db::Database>(
        self,
        summary: Arc<Mutex<Summary>>,
        vm: &mut revm::Evm<EXT,DB>,
        mode: Mode,
        test_group: Option<String>,
        name_prefix: String,
        evm_builds: &HashMap<String, Build, RandomState>,
    ) {
        let name = format!("{}[#deployer:{}]", name_prefix, self.identifier);

        //vm.populate_storage(self.storage.inner);
        let build = evm_builds.get(self.identifier.as_str()).expect("Always valid");
        let mut deploy_code = build.deploy_build.bytecode.to_owned();
        deploy_code.extend(self.calldata.inner.clone());

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
        env.tx.data = revm::primitives::Bytes::from(deploy_code);
        env.tx.value = revm::primitives::U256::from(self.value.unwrap_or_default());
        env.tx.access_list = vec![];
        //env.tx.transact_to = ;

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
                        Bytes::from_hex(address.unwrap()).unwrap()
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
