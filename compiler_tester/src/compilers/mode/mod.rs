//!
//! The compiler mode.
//!

pub mod llvm;
pub mod solidity;
pub mod vyper;
pub mod yul;
pub mod zkevm;

use self::llvm::Mode as LLVMMode;
use self::solidity::Mode as SolidityMode;
use self::vyper::Mode as VyperMode;
use self::yul::Mode as YulMode;
use self::zkevm::Mode as zkEVMMode;

///
/// The compiler mode.
///
#[derive(Debug, Clone)]
#[allow(non_camel_case_types)]
#[allow(clippy::upper_case_acronyms)]
pub enum Mode {
    /// The `Yul` mode.
    Yul(YulMode),
    /// The `Solidity` mode.
    Solidity(SolidityMode),
    /// The `LLVM` mode.
    LLVM(LLVMMode),
    /// The `zkEVM` mode.
    zkEVM(zkEVMMode),
    /// The `Vyper` mode.
    Vyper(VyperMode),
}

impl Mode {
    ///
    /// Checks if the self is compatible with version filter.
    ///
    pub fn check_version(&self, versions: &semver::VersionReq) -> bool {
        let version = match self {
            Mode::Solidity(mode) => &mode.solc_version,
            Mode::Vyper(mode) => &mode.vyper_version,
            _ => return false,
        };
        versions.matches(version)
    }
}

impl From<YulMode> for Mode {
    fn from(inner: YulMode) -> Self {
        Self::Yul(inner)
    }
}

impl From<SolidityMode> for Mode {
    fn from(inner: SolidityMode) -> Self {
        Self::Solidity(inner)
    }
}

impl From<LLVMMode> for Mode {
    fn from(inner: LLVMMode) -> Self {
        Self::LLVM(inner)
    }
}

impl From<zkEVMMode> for Mode {
    fn from(inner: zkEVMMode) -> Self {
        Self::zkEVM(inner)
    }
}

impl From<VyperMode> for Mode {
    fn from(inner: VyperMode) -> Self {
        Self::Vyper(inner)
    }
}

impl std::fmt::Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Yul(inner) => write!(f, "{inner}"),
            Self::Solidity(inner) => write!(f, "{inner}"),
            Self::LLVM(inner) => write!(f, "{inner}"),
            Self::zkEVM(inner) => write!(f, "{inner}"),
            Self::Vyper(inner) => write!(f, "{inner}"),
        }
    }
}
