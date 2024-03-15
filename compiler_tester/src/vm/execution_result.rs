//!
//! The VM execution result.
//!

use crate::test::case::input::output::Output;
use crate::vm::evm::output::Output as EVMOutput;

///
/// The VM execution result.
///
#[derive(Debug, Clone)]
pub struct ExecutionResult {
    /// The actual snapshot result data.
    pub output: Output,
    /// The number of executed cycles.
    pub cycles: usize,
    /// The amount of gas used.
    pub gas: u32,
}

impl ExecutionResult {
    ///
    /// A shortcut constructor.
    ///
    pub fn new(output: Output, cycles: usize, gas: u32) -> Self {
        Self {
            output,
            cycles,
            gas,
        }
    }
}

impl From<&zkevm_tester::runners::compiler_tests::VmSnapshot> for ExecutionResult {
    fn from(snapshot: &zkevm_tester::runners::compiler_tests::VmSnapshot) -> Self {
        Self {
            output: Output::from(snapshot),
            cycles: snapshot.num_cycles_used,
            gas: snapshot.num_ergs_used,
        }
    }
}

impl From<EVMOutput> for ExecutionResult {
    fn from(output: EVMOutput) -> Self {
        Self {
            output: Output::from(output),
            cycles: 0,
            gas: 0,
        }
    }
}
