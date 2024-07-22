//!
//! The contract call input variant.
//!

use std::str::FromStr;
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
use crate::vm::eravm::system_context::SystemContext;
use crate::vm::eravm::EraVM;
use crate::vm::evm::EVM;

use crate::vm::revm::revm_type_conversions::revm_bytes_to_vec_value;
use crate::vm::revm::revm_type_conversions::transform_success_output;
use crate::vm::revm::Revm;

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
    /// Runs the call on EVM emulator.
    ///
    pub fn run_evm_emulator(
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
    pub fn run_revm(
        self,
        summary: Arc<Mutex<Summary>>,
        vm: Revm,
        mode: Mode,
        test_group: Option<String>,
        name_prefix: String,
        index: usize,
        evm_version: Option<EVMVersion>,
    ) -> Revm {
        let name = format!("{}[{}:{}]", name_prefix, self.name, index);

        // On revm we can't send a tx with a tx_origin different from the tx_sender,
        // this specific test expects tx_origin to be that value, so we change the sender
        let mut caller = self.caller;
        if name_prefix == "solidity/test/libsolidity/semanticTests/state/tx_origin.sol" {
            caller = web3::types::Address::from_str("0x9292929292929292929292929292929292929292")
                .unwrap();
        }

        let rich_addresses = SystemContext::get_rich_addresses();
        let vm = if rich_addresses.contains(&caller) {
            vm.update_runtime_balance(caller)
        } else {
            vm
        };

        let mut vm = vm.fill_runtime_new_transaction(self.address, caller, self.calldata.clone(), self.value, evm_version);

        vm = vm.update_balance_if_lack_of_funds(caller);

        let result = match vm.state.transact_commit() {
            Ok(result) => result,
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
        let output = match result {
            ExecutionResult::Success {
                reason: _,
                gas_used: _,
                gas_refunded: _,
                logs,
                output,
            } => {
                vm = if !SystemContext::get_rich_addresses().contains(&caller) {
                    vm.non_rich_update_balance(caller)
                } else {
                    vm
                };
                transform_success_output(output, logs)
            }
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
