//!
//! The compiler tester Yul mode.
//!

use crate::llvm_options::LLVMOptions;

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
    ///
    /// A shortcut constructor.
    ///
    pub fn new(mut llvm_optimizer_settings: compiler_llvm_context::OptimizerSettings) -> Self {
        let llvm_options = LLVMOptions::get();
        llvm_optimizer_settings.is_verify_each_enabled = llvm_options.is_verify_each_enabled();
        llvm_optimizer_settings.is_debug_logging_enabled = llvm_options.is_debug_logging_enabled();

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
        write!(f, "{}", self.llvm_optimizer_settings)
    }
}
