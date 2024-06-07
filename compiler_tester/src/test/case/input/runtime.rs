//!
//! The contract call input variant.
//!

use std::sync::Arc;
use std::sync::Mutex;

use crate::compilers::mode::Mode;
use crate::summary::Summary;
use crate::test::case::input::calldata::Calldata;
use crate::test::case::input::output::Output;
use crate::test::case::input::storage::Storage;
use crate::vm::eravm::EraVM;
use crate::vm::evm::EVM;

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
        let mut result = match vm.execute::<M>(
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
        let gas = if let Some(benchmark_analyzer::Benchmark::EVM_INTERPRETER_GROUP_NAME) =
            test_group.as_deref()
        {
            if result.output.return_data.is_empty() {
                Summary::invalid(
                    summary,
                    Some(mode),
                    name,
                    "EVM interpreter gas usage value not found",
                );
                return;
            }
            result
                .output
                .return_data
                .remove(0)
                .unwrap_certain_as_ref()
                .as_u64()
                - EraVM::EVM_INTERPRETER_GAS_OVERHEAD
        } else {
            0
        };

        if result.output == self.expected {
            Summary::passed_runtime(
                summary,
                mode,
                name,
                test_group,
                result.cycles,
                result.ergs,
                gas,
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
    /// Runs the call on EVM.
    ///
    pub fn run_evm(
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
        let result = match vm.execute_evm::<M>(
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
