//!
//! The EVM deploy contract call input variant.
//!

use std::sync::Arc;
use std::sync::Mutex;

use revm::context::result::ExecutionResult;
use revm::ExecuteCommitEvm;

use crate::summary::Summary;
use crate::test::case::input::calldata::Calldata;
use crate::test::case::input::identifier::InputIdentifier;
use crate::test::case::input::output::Output;
use crate::test::case::input::storage::Storage;
use crate::test::description::TestDescription;
use crate::test::InputContext;
use crate::vm::eravm::deployers::EraVMDeployer;
use crate::vm::eravm::system_context::SystemContext;
use crate::vm::eravm::EraVM;

use crate::vm::revm::revm_type_conversions::revm_bytes_to_vec_value;
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
    pub fn run_revm(self, summary: Arc<Mutex<Summary>>, vm: &mut REVM, context: InputContext<'_>) {
        let input_index = context.selector;
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

        let tx = REVM::new_deploy_transaction(self.caller, self.value, code);

        let initial_balance = (web3::types::U256::from(1) << 100)
            + web3::types::U256::from(self.value.unwrap_or_default());
        vm.set_account(&self.caller, initial_balance);

        vm.evm.block.number = revm::primitives::U256::from(input_index + 1);
        vm.evm.block.timestamp = revm::primitives::U256::from(
            ((input_index + 1) as u128) * SystemContext::BLOCK_TIMESTAMP_EVM_STEP,
        );

        let result = match vm.evm.transact_commit(tx) {
            Ok(result) => result,
            Err(error) => {
                Summary::invalid(summary.clone(), test, error);
                return;
            }
        };

        let (output, gas, halt_reason) = match result {
            ExecutionResult::Success {
                reason: _,
                gas_used,
                gas_refunded: _,
                logs,
                output,
            } => ((output, logs).into(), gas_used, None),
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
        } else if let Some(error) = halt_reason {
            Summary::invalid(summary, test, format!("{error:?}"));
        } else {
            Summary::failed(summary, test, self.expected, output, calldata);
        }
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
