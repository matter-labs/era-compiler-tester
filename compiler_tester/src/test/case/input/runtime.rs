//!
//! The contract call input variant.
//!

use std::str::FromStr;
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
use crate::test::context::input::InputContext;
use crate::test::description::TestDescription;
use crate::vm::eravm::system_context::SystemContext;
use crate::vm::eravm::EraVM;

use crate::vm::revm::revm_type_conversions::revm_bytes_to_vec_value;
use crate::vm::revm::revm_type_conversions::transform_success_output;
use crate::vm::revm::REVM;

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
        context: InputContext<'_>,
    ) {
        let group = context.case_context.group.clone();
        let input_index = context.selector;
        let test = TestDescription::from_context(
            context,
            Self::select_input_identifier(self.name, input_index),
        );
        let name = test.selector.to_string();
        vm.populate_storage(self.storage.inner);
        let vm_function = match group.as_deref() {
            Some(benchmark_analyzer::TEST_GROUP_EVM_INTERPRETER) => {
                EraVM::execute_evm_interpreter::<M>
            }
            _ => EraVM::execute::<M>,
        };
        let result = match vm_function(
            vm,
            name,
            self.address,
            self.caller,
            self.value,
            self.calldata.inner.clone(),
            None,
        ) {
            Ok(result) => result,
            Err(error) => {
                Summary::invalid(summary, test, error);
                return;
            }
        };

        if result.output == self.expected {
            Summary::passed_runtime(summary, test, result.cycles, result.ergs, result.gas);
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

    fn select_input_identifier(name: String, input_index: usize) -> InputIdentifier {
        match name.as_str() {
            "#fallback" => InputIdentifier::Fallback { input_index },
            _ => InputIdentifier::Runtime { input_index, name },
        }
    }

    ///
    /// Runs the call on REVM.
    ///
    pub fn run_revm<'b>(
        self,
        summary: Arc<Mutex<Summary>>,
        vm: REVM<'b>,
        evm_version: Option<EVMVersion>,
        context: InputContext<'_>,
    ) -> REVM<'b> {
        let input_index = context.selector;
        let test = TestDescription::from_context(
            context,
            InputIdentifier::Runtime {
                input_index,
                name: self.name,
            },
        );

        // On revm we can't send a tx with a tx_origin different from the tx_sender,
        // this specific test expects tx_origin to be that value, so we change the sender
        let mut caller = self.caller;
        if test.selector.path == "solidity/test/libsolidity/semanticTests/state/tx_origin.sol" {
            caller = web3::types::Address::from_str("0x9292929292929292929292929292929292929292")
                .unwrap();
        }

        let rich_addresses = SystemContext::get_rich_addresses();
        let vm = if rich_addresses.contains(&caller) {
            vm.update_runtime_balance(caller)
        } else {
            vm
        };

        let mut vm = vm.fill_runtime_new_transaction(
            self.address,
            caller,
            self.calldata.clone(),
            self.value,
            evm_version,
            input_index,
        );
        vm = vm.update_balance_if_lack_of_funds(caller);

        let result = match vm.state.transact_commit() {
            Ok(result) => result,
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
            } => {
                vm = if !SystemContext::get_rich_addresses().contains(&caller) {
                    vm.non_rich_update_balance(caller)
                } else {
                    vm
                };
                (transform_success_output(output, logs), gas_used, None)
            }
            ExecutionResult::Revert { gas_used, output } => {
                let return_data_value = revm_bytes_to_vec_value(output);
                (Output::new(return_data_value, true, vec![]), gas_used, None)
            }
            ExecutionResult::Halt { reason, gas_used } => {
                (Output::new(vec![], true, vec![]), gas_used, Some(reason))
            }
        };

        if output == self.expected {
            Summary::passed_runtime(summary, test, 0, 0, gas);
        } else if let Some(error) = error {
            Summary::invalid(summary, test, format!("{error:?}"));
        } else {
            Summary::failed(summary, test, self.expected, output, self.calldata.inner);
        };
        vm
    }

    ///
    /// Runs the call on EVM interpreter.
    ///
    pub fn run_evm_interpreter<const M: bool>(
        self,
        summary: Arc<Mutex<Summary>>,
        vm: &mut EraVM,
        context: InputContext<'_>,
    ) {
        let input_index = context.selector;
        let test = TestDescription::from_context(
            context,
            InputIdentifier::Runtime {
                input_index,
                name: self.name,
            },
        );
        let name = test.selector.to_string();
        vm.populate_storage(self.storage.inner);
        let result = match vm.execute_evm_interpreter::<M>(
            name,
            self.address,
            self.caller,
            self.value,
            self.calldata.inner.clone(),
            None,
        ) {
            Ok(result) => result,
            Err(error) => {
                Summary::invalid(summary, test, error);
                return;
            }
        };

        if result.output == self.expected {
            Summary::passed_runtime(summary, test, result.cycles, result.ergs, result.gas);
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
