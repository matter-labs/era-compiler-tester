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
        /// The amount of gas used.
        gas: u32,
    },
    /// The contract call.
    Runtime {
        /// The number of execution cycles.
        cycles: usize,
        /// The amount of gas used.
        gas: u32,
    },
    /// The special function call.
    Special,
}
