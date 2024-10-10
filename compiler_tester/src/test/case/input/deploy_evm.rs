//!
//! The EVM deploy contract call input variant.
//!

use std::sync::Arc;
use std::sync::Mutex;

use revm::primitives::EVMError;
use revm::primitives::ExecutionResult;
use solidity_adapter::EVMVersion;

use crate::compilers::mode::Mode;
use crate::summary::Summary;
use crate::test::case::input::calldata::Calldata;
use crate::test::case::input::output::Output;
use crate::test::case::input::storage::Storage;
use crate::vm::eravm::deployers::EraVMDeployer;
use crate::vm::eravm::EraVM;
use crate::vm::evm::EVM;

use crate::vm::revm::revm_type_conversions::revm_bytes_to_vec_value;
use crate::vm::revm::revm_type_conversions::transform_success_output;
use crate::vm::revm::Revm;

///
/// The EVM deploy contract call input variant.
///
#[derive(Debug, Clone)]
pub struct DeployEVM {
    /// The contract identifier.
    identifier: String,
    /// The contract deploy code.
    deploy_code: Vec<u8>,
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
        deploy_code: Vec<u8>,
        calldata: Calldata,
        caller: web3::types::Address,
        value: Option<u128>,
        storage: Storage,
        expected: Output,
    ) -> Self {
        Self {
            identifier,
            deploy_code,
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
    pub fn run_revm<'a>(
        self,
        summary: Arc<Mutex<Summary>>,
        vm: Revm<'a>,
        mode: Mode,
        test_group: Option<String>,
        name_prefix: String,
        evm_version: Option<EVMVersion>,
    ) -> Revm<'a> {
        let name = format!("{}[#deployer:{}]", name_prefix, self.identifier);

        let size = self.deploy_code.len();

        let vm = vm.update_deploy_balance(&self.caller);
        let mut vm =
            vm.fill_deploy_new_transaction(self.caller, self.value, evm_version, self.deploy_code);

        let result = match vm.state.transact_commit() {
            Ok(res) => res,
            Err(error) => {
                match error {
                    EVMError::Transaction(error) => {
                        Summary::invalid(
                            summary.clone(),
                            Some(mode.clone()),
                            name.clone(),
                            format!("Error on Transaction: {error:?}"),
                        );
                    }
                    EVMError::Header(error) => {
                        Summary::invalid(
                            summary.clone(),
                            Some(mode.clone()),
                            name.clone(),
                            format!("Error on Header: {error:?}"),
                        );
                    }
                    EVMError::Database(_error) => {
                        Summary::invalid(
                            summary.clone(),
                            Some(mode.clone()),
                            name.clone(),
                            "Error on Database",
                        );
                    }
                    EVMError::Custom(error) => {
                        Summary::invalid(
                            summary.clone(),
                            Some(mode.clone()),
                            name.clone(),
                            format!("Error on Custom: {error:?}"),
                        );
                    }
                    EVMError::Precompile(error) => {
                        Summary::invalid(
                            summary.clone(),
                            Some(mode.clone()),
                            name.clone(),
                            format!("Error on Precompile: {error:?}"),
                        );
                    }
                }
                return vm;
            }
        };

        let (output, gas, error) = match result {
            ExecutionResult::Success {
                reason: _,
                gas_used,
                gas_refunded: _,
                logs,
                output,
            } => (transform_success_output(output, logs), gas_used, None),
            ExecutionResult::Revert { gas_used, output } => {
                let return_data_value = revm_bytes_to_vec_value(output);
                (Output::new(return_data_value, true, vec![]), gas_used, None)
            }
            ExecutionResult::Halt { reason, gas_used } => {
                (Output::new(vec![], true, vec![]), gas_used, Some(reason))
            }
        };

        if output == self.expected {
            Summary::passed_deploy(summary, mode, name, test_group, size, 0, 0, gas);
        } else if let Some(error) = error {
            Summary::invalid(summary, Some(mode), name, format!("{error:?}"));
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

        vm
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

        let size = self.deploy_code.len();

        vm.populate_storage(self.storage.inner);
        let result = match deployer.deploy_evm::<M>(
            name.clone(),
            self.caller,
            self.deploy_code,
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
