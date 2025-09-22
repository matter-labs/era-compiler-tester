//!
//! The contract call input variant.
//!

use std::sync::Arc;
use std::sync::Mutex;

use revm::context::result::EVMError;
use revm::context::result::ExecutionResult;
use revm::context::result::InvalidTransaction;
use revm::context::Host;
use revm::ExecuteCommitEvm;

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
use crate::vm::revm::revm_type_conversions::web3_address_to_revm_address;
use crate::vm::revm::revm_type_conversions::web3_h256_to_revm_u256;
use crate::vm::revm::revm_type_conversions::web3_u256_to_revm_u256;
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
    pub fn run_revm(self, summary: Arc<Mutex<Summary>>, vm: &mut REVM, context: InputContext<'_>) {
        let input_index = context.selector;
        let test = TestDescription::from_context(
            context,
            InputIdentifier::Runtime {
                input_index,
                name: self.name,
            },
        );

        let balance = web3::types::U256::from(self.value.unwrap_or_default())
            + web3::types::U256::from(1267650600228229401496703205376u128);
        vm.update_balance(self.caller, balance);
        if input_index == 1 {
            for address in SystemContext::get_rich_addresses() {
                let balance = web3::types::U256::from(1267650600228229401496703205376u128);
                vm.update_balance(address, balance);
            }
        }
        for ((address, key), value) in self.storage.inner.into_iter() {
            let address = web3_address_to_revm_address(&address);

            if vm.evm.ctx.load_account_code(address).is_none() {
                continue;
            }
            vm.evm.ctx.sstore(
                address,
                web3_u256_to_revm_u256(key),
                web3_h256_to_revm_u256(value),
            );
        }
        let tx = REVM::new_runtime_transaction(
            self.address,
            self.caller,
            self.calldata.clone(),
            self.value,
        );

        let result = match vm.evm.transact_commit(tx) {
            Ok(result) => result,
            Err(error) => {
                if let EVMError::Transaction(InvalidTransaction::LackOfFundForMaxFee {
                    ref fee,
                    ..
                }) = error
                {
                    vm.update_balance_if_lack_of_funds(self.caller, **fee);
                }
                Summary::invalid(summary.clone(), test, error);
                return;
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
                if !SystemContext::get_rich_addresses().contains(&self.caller) {
                    vm.non_rich_update_balance(self.caller);
                }
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
        }
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
