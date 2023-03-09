//!
//! The Solidity subprocess compiler mode.
//!

///
/// The Solidity subprocess compiler mode.
///
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SubprocessMode {
    /// The Solidity compiler version.
    pub version: semver::Version,
    /// The Solidity compiler output type.
    pub pipeline: compiler_solidity::SolcPipeline,
    /// Whether to run the Solidity compiler optimizer.
    pub optimize: bool,
}

impl SubprocessMode {
    ///
    /// A shortcut constructor.
    ///
    pub fn new(
        solc_version: semver::Version,
        solc_pipeline: compiler_solidity::SolcPipeline,
        solc_optimize: bool,
    ) -> Self {
        Self {
            version: solc_version,
            pipeline: solc_pipeline,
            optimize: solc_optimize,
        }
    }
}
