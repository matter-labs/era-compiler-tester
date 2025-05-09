//!
//! The VM execution result.
//!

use crate::test::case::input::output::Output;

///
/// The VM execution result.
///
#[derive(Debug, Clone)]
pub struct ExecutionResult {
    /// The VM snapshot execution result.
    pub output: Output,
    /// The number of executed cycles.
    pub cycles: u64,
    /// The number of EraVM ergs used.
    pub ergs: u64,
    /// The number of gas used.
    pub gas: u64,
}

impl ExecutionResult {
    ///
    /// A shortcut constructor.
    ///
    pub fn new(output: Output, cycles: u64, ergs: u64, gas: u64) -> Self {
        Self {
            output,
            cycles,
            ergs,
            gas,
        }
    }
}

impl From<zkevm_tester::compiler_tests::VmSnapshot> for ExecutionResult {
    fn from(snapshot: zkevm_tester::compiler_tests::VmSnapshot) -> Self {
        let cycles = snapshot.num_cycles_used as u64;
        let ergs = snapshot.num_ergs_used as u64;

        Self {
            output: Output::from(snapshot),
            cycles,
            ergs,
            gas: 0,
        }
    }
}
