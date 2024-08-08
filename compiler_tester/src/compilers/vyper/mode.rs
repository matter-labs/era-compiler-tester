//!
//! The compiler tester Vyper mode.
//!

use crate::compilers::mode::llvm_options::LLVMOptions;

use crate::compilers::mode::Mode as ModeWrapper;

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
    pub llvm_optimizer_settings: era_compiler_llvm_context::OptimizerSettings,
}

impl Mode {
    ///
    /// A shortcut constructor.
    ///
    pub fn new(
        vyper_version: semver::Version,
        vyper_optimize: bool,
        mut llvm_optimizer_settings: era_compiler_llvm_context::OptimizerSettings,
    ) -> Self {
        let llvm_options = LLVMOptions::get();
        llvm_optimizer_settings.enable_fallback_to_size();
        llvm_optimizer_settings.is_verify_each_enabled = llvm_options.is_verify_each_enabled();
        llvm_optimizer_settings.is_debug_logging_enabled = llvm_options.is_debug_logging_enabled();

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

    ///
    /// Checks if the mode is compatible with the source code pragmas.
    ///
    pub fn check_pragmas(&self, sources: &[(String, String)]) -> bool {
        sources.iter().all(|(_, source_code)| {
            match source_code.lines().find_map(|line| {
                let mut split = line.split_whitespace();
                if let (Some("#"), Some("@version"), Some(version)) =
                    (split.next(), split.next(), split.next())
                {
                    semver::VersionReq::parse(version).ok()
                } else {
                    None
                }
            }) {
                Some(pragma_version_req) => pragma_version_req.matches(&self.vyper_version),
                None => true,
            }
        })
    }
}

impl std::fmt::Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "V{}{} {}",
            if self.vyper_optimize { '+' } else { '-' },
            self.llvm_optimizer_settings,
            self.vyper_version,
        )
    }
}
