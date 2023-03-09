//!
//! The compiler tester Solidity mode.
//!

use super::Mode as ModeWrapper;

///
/// The compiler tester Solidity mode.
///
#[derive(Debug, Clone)]
pub struct Mode {
    /// The Solidity compiler version.
    pub solc_version: semver::Version,
    /// The Solidity compiler output type.
    pub solc_pipeline: compiler_solidity::SolcPipeline,
    /// Whether to run the Solidity compiler optimizer.
    pub solc_optimize: bool,
    /// The optimizer settings.
    pub llvm_optimizer_settings: compiler_llvm_context::OptimizerSettings,
}

impl Mode {
    /// The language name.
    pub const LANGUAGE: &'static str = "Solidity";

    ///
    /// A shortcut constructor.
    ///
    pub fn new(
        solc_version: semver::Version,
        solc_pipeline: compiler_solidity::SolcPipeline,
        solc_optimize: bool,
        llvm_optimizer_settings: compiler_llvm_context::OptimizerSettings,
    ) -> Self {
        Self {
            solc_version,
            solc_pipeline,
            solc_optimize,
            llvm_optimizer_settings,
        }
    }

    ///
    /// Unwrap mode.
    ///
    /// # Panics
    ///
    /// Will panic if the inner is non-Solidity mode.
    ///
    pub fn unwrap(mode: &ModeWrapper) -> &Self {
        match mode {
            ModeWrapper::Solidity(mode) => mode,
            _ => panic!("Non-Solidity mode"),
        }
    }
}

impl std::fmt::Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:>8} {}{}{} {}",
            Self::LANGUAGE,
            match self.solc_pipeline {
                compiler_solidity::SolcPipeline::Yul => "Y",
                compiler_solidity::SolcPipeline::EVMLA => "E",
            },
            if self.solc_optimize { '+' } else { '-' },
            self.llvm_optimizer_settings,
            self.solc_version,
        )
    }
}
