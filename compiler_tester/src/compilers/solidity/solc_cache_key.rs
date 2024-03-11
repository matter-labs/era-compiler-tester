//!
//! The Solidity subprocess compiler cache key.
//!

///
/// The Solidity subprocess compiler cache key.
///
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SolcCacheKey {
    /// The test path.
    pub test_path: String,
    /// The Solidity compiler version.
    pub version: semver::Version,
    /// The Solidity compiler output type.
    pub pipeline: era_compiler_solidity::SolcPipeline,
    /// Whether to enable the EVMLA codegen via Yul IR.
    pub via_ir: bool,
    /// Whether to run the Solidity compiler optimizer.
    pub optimize: bool,
}

impl SolcCacheKey {
    ///
    /// A shortcut constructor.
    ///
    pub fn new(
        test_path: String,
        version: semver::Version,
        pipeline: era_compiler_solidity::SolcPipeline,
        via_ir: bool,
        optimize: bool,
    ) -> Self {
        Self {
            test_path,
            version,
            pipeline,
            via_ir,
            optimize,
        }
    }
}
