//!
//! The compiler tester Yul mode.
//!

use crate::compilers::mode::llvm_options::LLVMOptions;

use crate::compilers::mode::Mode as ModeWrapper;

///
/// The compiler tester Yul mode.
///
#[derive(Debug, Clone)]
pub struct Mode {
    /// The optimizer settings.
    pub llvm_optimizer_settings: era_compiler_llvm_context::OptimizerSettings,
    /// Whether the EraVM extensions are enabled.
    pub enable_eravm_extensions: bool,
}

impl Mode {
    ///
    /// A shortcut constructor.
    ///
    pub fn new(
        mut llvm_optimizer_settings: era_compiler_llvm_context::OptimizerSettings,
        enable_eravm_extensions: bool,
    ) -> Self {
        let llvm_options = LLVMOptions::get();
        llvm_optimizer_settings.enable_fallback_to_size();
        llvm_optimizer_settings.is_verify_each_enabled = llvm_options.is_verify_each_enabled();
        llvm_optimizer_settings.is_debug_logging_enabled = llvm_options.is_debug_logging_enabled();

        Self {
            llvm_optimizer_settings,
            enable_eravm_extensions,
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
