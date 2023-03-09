//!
//! The zkEVM execution result.
//!

use crate::test::case::input::output::Output;

///
/// The zkEVM execution result.
///
#[derive(Debug, Clone)]
pub struct ExecutionResult {
    /// The actual snapshot result data.
    pub output: Output,
    /// The number of executed cycles.
    pub cycles: usize,
}

impl ExecutionResult {
    ///
    /// A shortcut constructor.
    ///
    pub fn new(output: Output, cycles: usize) -> Self {
        Self { output, cycles }
    }
}

impl From<&zkevm_tester::runners::compiler_tests::VmSnapshot> for ExecutionResult {
    fn from(snapshot: &zkevm_tester::runners::compiler_tests::VmSnapshot) -> Self {
        Self {
            output: Output::from(snapshot),
            cycles: snapshot.num_cycles_used,
        }
    }
}
