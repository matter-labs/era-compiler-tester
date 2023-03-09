//!
//! The compiler tester Vyper mode.
//!

use super::Mode as ModeWrapper;

///
/// The compiler tester Vyper mode.
///
#[derive(Debug, Clone)]
pub struct Mode {
    /// The Vyper compiler version.
    pub vyper_version: semver::Version,
    /// Whether to run the Vyper compiler optimizer.
    pub vyper_optimize: bool,
    /// The optimizer settings.
    pub llvm_optimizer_settings: compiler_llvm_context::OptimizerSettings,
}

impl Mode {
    /// The language name.
    pub const LANGUAGE: &'static str = "Vyper";

    ///
    /// A shortcut constructor.
    ///
    pub fn new(
        vyper_version: semver::Version,
        vyper_optimize: bool,
        llvm_optimizer_settings: compiler_llvm_context::OptimizerSettings,
    ) -> Self {
        Self {
            vyper_version,
            vyper_optimize,
            llvm_optimizer_settings,
        }
    }

    ///
    /// Unwrap mode.
    ///
    /// # Panics
    ///
    /// Will panic if the inner is non-Vyper mode.
    ///
    pub fn unwrap(mode: &ModeWrapper) -> &Self {
        match mode {
            ModeWrapper::Vyper(mode) => mode,
            _ => panic!("Non-Vyper mode"),
        }
    }
}

impl std::fmt::Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:>8} V{}{} {}",
            Self::LANGUAGE,
            if self.vyper_optimize { '+' } else { '-' },
            self.llvm_optimizer_settings,
            self.vyper_version,
        )
    }
}
