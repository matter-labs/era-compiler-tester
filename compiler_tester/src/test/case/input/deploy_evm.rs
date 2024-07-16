//!
//! The EVM deploy contract call input variant.
//!

use std::collections::HashMap;
use std::hash::RandomState;
use std::sync::Arc;
use std::sync::Mutex;

use revm::primitives::EVMError;
use revm::primitives::Env;
use revm::primitives::ExecutionResult;
use revm::primitives::TxKind;
use revm::primitives::B256;
use revm::primitives::U256;
use revm::Database;
use revm::State;
use solidity_adapter::EVMVersion;

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

use super::revm_type_conversions::revm_bytes_to_vec_value;
use super::revm_type_conversions::transform_success_output;
use super::revm_type_conversions::web3_address_to_revm_address;
use super::revm_type_conversions::web3_u256_to_revm_address;
use super::revm_type_conversions::web3_u256_to_revm_u256;
use std::str::FromStr;

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
    /// Runs the deploy transaction on EVM emulator.
    ///
    pub fn run_evm_emulator(
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
        evm_version: Option<EVMVersion>,
    ) -> revm::Evm<'a, EXT, State<DB>> {
        let name = format!("{}[#deployer:{}]", name_prefix, self.identifier);

        let build = evm_builds
            .get(self.identifier.as_str())
            .expect("Always valid");
        let mut deploy_code = build.deploy_build.bytecode.to_owned();
        deploy_code.extend(self.calldata.inner.clone());

        let vm = self.update_balance(vm);

        let mut new_vm = vm
            .modify()
            .modify_env(|env| {
                let evm_context = SystemContext::get_constants_evm(evm_version);
                env.cfg.chain_id = evm_context.chain_id;
                env.block.number = U256::from(evm_context.block_number);
                let coinbase = web3::types::U256::from_str_radix(evm_context.coinbase, 16).unwrap();
                env.block.coinbase = web3_u256_to_revm_address(coinbase);
                env.block.timestamp = U256::from(evm_context.block_timestamp);
                env.block.gas_limit = U256::from(evm_context.block_gas_limit);
                env.block.basefee = U256::from(evm_context.base_fee);
                let block_difficulty =
                    web3::types::U256::from_str_radix(evm_context.block_difficulty, 16).unwrap();
                env.block.difficulty = web3_u256_to_revm_u256(block_difficulty);
                env.block.prevrandao = Some(B256::from(env.block.difficulty));
                env.tx.gas_price = U256::from(0xb2d05e00_u32);
                env.tx.gas_limit = evm_context.block_gas_limit;
                env.tx.access_list = vec![];
                env.tx.caller = web3_address_to_revm_address(&self.caller);
                env.tx.data = revm::primitives::Bytes::from(deploy_code);
                env.tx.value = revm::primitives::U256::from(self.value.unwrap_or_default());
                env.tx.transact_to = TxKind::Create;
            })
            .build();

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
                reason: _,
                gas_used: _,
                gas_refunded: _,
                logs,
                output,
            } => transform_success_output(output, logs),
            ExecutionResult::Revert {
                gas_used: _,
                output,
            } => {
                let return_data_value = revm_bytes_to_vec_value(output);
                Output::new(return_data_value, true, vec![])
            }
            ExecutionResult::Halt {
                reason: _,
                gas_used: _,
            } => Output::new(vec![], true, vec![]),
        };

        if output == self.expected {
            Summary::passed_deploy(summary, mode, name, test_group, 0, 0, 0, 0);
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

    /// All accounts used to deploy the test contracts should have a balance of U256::MAX
    fn update_balance<'a, EXT, DB: Database>(
        &self,
        mut vm: revm::Evm<'a, EXT, State<DB>>,
    ) -> revm::Evm<'a, EXT, State<DB>> {
        let address = web3_address_to_revm_address(&self.caller);
        let nonce = match vm.db_mut().basic(address) {
            Ok(Some(acc)) => acc.nonce,
            _ => 1,
        };
        let acc_info = revm::primitives::AccountInfo {
            balance: U256::MAX,
            code_hash: revm::primitives::KECCAK_EMPTY,
            code: None,
            nonce,
        };
        let mut vm = vm
            .modify()
            .modify_db(|db| {
                db.insert_account(address, acc_info.clone());
            })
            .modify_env(|env| env.clone_from(&Box::new(Env::default())))
            .build();
        vm.transact_commit().ok(); // Even if TX fails, the balance update will be committed
        vm
    }
}
