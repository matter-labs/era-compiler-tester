//!
//! The Solidity compiler cache key.
//!

///
/// The Solidity compiler cache key.
///
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CacheKey {
    /// The test path.
    pub test_path: String,
    /// The Solidity compiler version.
    pub version: semver::Version,
    /// The Solidity compiler output type.
    pub codegen: era_compiler_solidity::SolcCodegen,
    /// Whether to enable the EVMLA codegen via Yul IR.
    pub via_ir: bool,
    /// Whether to run the Solidity compiler optimizer.
    pub optimize: bool,
}

impl CacheKey {
    ///
    /// A shortcut constructor.
    ///
    pub fn new(
        test_path: String,
        version: semver::Version,
        codegen: era_compiler_solidity::SolcCodegen,
        via_ir: bool,
        optimize: bool,
    ) -> Self {
        Self {
            test_path,
            version,
            codegen,
            via_ir,
            optimize,
        }
    }
}
