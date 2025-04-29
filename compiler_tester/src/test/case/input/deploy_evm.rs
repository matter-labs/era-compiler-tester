//!
//! The EVM deploy contract call input variant.
//!

use std::sync::Arc;
use std::sync::Mutex;

use revm::primitives::EVMError;
use revm::primitives::ExecutionResult;
use solidity_adapter::EVMVersion;

use crate::summary::Summary;
use crate::test::case::input::calldata::Calldata;
use crate::test::case::input::identifier::InputIdentifier;
use crate::test::case::input::output::Output;
use crate::test::case::input::storage::Storage;
use crate::test::description::TestDescription;
use crate::test::InputContext;
use crate::vm::eravm::deployers::EraVMDeployer;
use crate::vm::eravm::EraVM;

use crate::vm::revm::revm_type_conversions::revm_bytes_to_vec_value;
use crate::vm::revm::revm_type_conversions::transform_success_output;
use crate::vm::revm::REVM;

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
    /// Runs the deploy transaction on native REVM.
    ///
    pub fn run_revm<'b>(
        self,
        summary: Arc<Mutex<Summary>>,
        vm: REVM<'b>,
        evm_version: Option<EVMVersion>,
        context: InputContext<'_>,
    ) -> REVM<'b> {
        let test = TestDescription::from_context(
            context,
            InputIdentifier::Deployer {
                contract_identifier: self.identifier.clone(),
            },
        );

        let size = self.deploy_code.len() as u64;
        let calldata = self.calldata.inner.clone();
        let mut code = self.deploy_code;
        code.extend(self.calldata.inner);

        let vm = vm.update_deploy_balance(&self.caller);
        let mut vm = vm.fill_deploy_new_transaction(self.caller, self.value, evm_version, code);

        let result = match vm.state.transact_commit() {
            Ok(res) => res,
            Err(error) => {
                let error_msg = match error {
                    EVMError::Transaction(error) => format!("Error on Transaction: {error:?}"),
                    EVMError::Header(error) => format!("Error on Header: {error:?}"),
                    EVMError::Database(_error) => "Error on Database".into(),
                    EVMError::Custom(error) => format!("Error on Custom: {error:?}"),
                    EVMError::Precompile(error) => format!("Error on Precompile: {error:?}"),
                };

                Summary::invalid(summary.clone(), test, error_msg);
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
            Summary::passed_deploy(summary, test, size, 0, 0, gas);
        } else if let Some(error) = error {
            Summary::invalid(summary, test, format!("{error:?}"));
        } else {
            Summary::failed(summary, test, self.expected, output, calldata);
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
        deployer: &mut D,
        context: InputContext<'_>,
    ) where
        D: EraVMDeployer,
    {
        let test = TestDescription::from_context(
            context,
            InputIdentifier::Deployer {
                contract_identifier: self.identifier.clone(),
            },
        );

        let name = test.selector.to_string();
        let size = self.deploy_code.len() as u64;

        vm.populate_storage(self.storage.inner);
        let result = match deployer.deploy_evm::<M>(
            name,
            self.caller,
            self.deploy_code,
            self.calldata.inner.clone(),
            self.value,
            vm,
        ) {
            Ok(result) => result,
            Err(error) => {
                Summary::invalid(summary, test, error);
                return;
            }
        };
        if result.output == self.expected {
            Summary::passed_deploy(summary, test, size, result.cycles, result.ergs, result.gas);
        } else {
            Summary::failed(
                summary,
                test,
                self.expected,
                result.output,
                self.calldata.inner,
            );
        }
    }
}
