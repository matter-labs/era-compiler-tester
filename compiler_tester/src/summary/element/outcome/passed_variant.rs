//!
//! The compiler tester summary element passed outcome variant.
//!

///
/// The compiler tester summary element passed outcome variant.
///
#[derive(Debug)]
pub enum PassedVariant {
    /// The contract deploy.
    Deploy {
        /// The contract size in instructions.
        size: usize,
        /// The number of execution cycles.
        cycles: usize,
        /// The number of used ergs.
        ergs: u64,
        /// The number of used gas.
        gas: u64,
    },
    /// The contract call.
    Runtime {
        /// The number of execution cycles.
        cycles: usize,
        /// The number of used ergs.
        ergs: u64,
        /// The number of used gas.
        gas: u64,
    },
    /// The special function call.
    Special,
}
