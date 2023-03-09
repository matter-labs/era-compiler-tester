//!
//! The compiler tester Yul mode.
//!

use super::Mode as ModeWrapper;

///
/// The compiler tester Yul mode.
///
#[derive(Debug, Clone)]
pub struct Mode {
    /// The optimizer settings.
    pub llvm_optimizer_settings: compiler_llvm_context::OptimizerSettings,
}

impl Mode {
    /// The language name.
    pub const LANGUAGE: &'static str = "Yul";

    ///
    /// A shortcut constructor.
    ///
    pub fn new(llvm_optimizer_settings: compiler_llvm_context::OptimizerSettings) -> Self {
        Self {
            llvm_optimizer_settings,
        }
    }

    ///
    /// Unwrap mode.
    ///
    /// # Panics
    ///
    /// Will panic if the inner is non-Yul mode.
    ///
    pub fn unwrap(mode: &ModeWrapper) -> &Self {
        match mode {
            ModeWrapper::Yul(mode) => mode,
            _ => panic!("Non-Yul mode"),
        }
    }
}

impl std::fmt::Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:>8} {}", Self::LANGUAGE, self.llvm_optimizer_settings,)
    }
}
