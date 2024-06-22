//!
//! The compiler tester upstream Yul mode.
//!

use crate::compilers::mode::Mode as ModeWrapper;

///
/// The compiler tester upstream Yul mode.
///
#[derive(Debug, Clone)]
pub struct Mode {
    /// The Solidity compiler version.
    pub solc_version: semver::Version,
    /// Whether to run the Solidity compiler optimizer.
    pub solc_optimize: bool,
}

impl Mode {
    ///
    /// A shortcut constructor.
    ///
    pub fn new(solc_version: semver::Version, solc_optimize: bool) -> Self {
        Self {
            solc_version,
            solc_optimize,
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
            ModeWrapper::YulUpstream(mode) => mode,
            _ => panic!("Non-Yul-upstream mode"),
        }
    }
}

impl std::fmt::Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Y{} {}",
            if self.solc_optimize { '+' } else { '-' },
            self.solc_version,
        )
    }
}
