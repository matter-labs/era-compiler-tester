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
    /// Checks if the mode is compatible with the filters.
    ///
    pub fn check_filters(&self, filters: &[String]) -> bool {
        filters.is_empty()
            || filters
                .iter()
                .any(|filter| self.normalize(filter).contains(filter))
    }

    ///
    /// Checks if the mode is compatible with the extended filters.
    /// The extended filter consists of 2 parts: mode substring and version range.
    ///
    pub fn check_extended_filters(&self, filters: &[String]) -> bool {
        if filters.is_empty() {
            return true;
        }
        for filter in filters.iter() {
            let mut split = filter.split_whitespace();
            let mode_filter = split.next().unwrap_or_default();
            let normalized_mode = self.normalize(mode_filter);
            if !normalized_mode.contains(mode_filter) {
                continue;
            }

            let version = match split.next() {
                Some(version) => version,
                None => return true,
            };
            if let Ok(version_req) = semver::VersionReq::parse(version) {
                if self.check_version(&version_req) {
                    return true;
                }
            }
        }
        false
    }

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

    ///
    /// Checks if the mode is compatible with the source code pragmas.
    ///
    pub fn check_pragmas(&self, sources: &[(String, String)]) -> bool {
        match self {
            Mode::Solidity(mode) => mode.check_pragmas(sources),
            Mode::Vyper(mode) => mode.check_pragmas(sources),
            _ => true,
        }
    }

    ///
    /// Checks if the mode is compatible with the Ethereum tests params.
    ///
    pub fn check_ethereum_tests_params(&self, params: &solidity_adapter::Params) -> bool {
        match self {
            Mode::Solidity(mode) => mode.check_ethereum_tests_params(params),
            _ => true,
        }
    }

    ///
    /// Normalizes the mode according to the filter.
    ///
    fn normalize(&self, filter: &str) -> String {
        let mut current = self.to_string();
        if filter.contains("Y*") {
            current = regex::Regex::new("Y[-+]")
                .expect("Always valid")
                .replace_all(current.as_str(), "Y*")
                .to_string();
        }
        if filter.contains("E*") {
            current = regex::Regex::new("E[-+]")
                .expect("Always valid")
                .replace_all(current.as_str(), "E*")
                .to_string();
        }
        if filter.contains("y*") {
            current = regex::Regex::new("y[-+]")
                .expect("Always valid")
                .replace_all(current.as_str(), "y*")
                .to_string();
        }
        if filter.contains("V*") {
            current = regex::Regex::new("V[-+]")
                .expect("Always valid")
                .replace_all(current.as_str(), "V*")
                .to_string();
        }
        if filter.contains("M^") {
            current = regex::Regex::new("M[3z]")
                .expect("Always valid")
                .replace_all(current.as_str(), "M^")
                .to_string();
        }
        if filter.contains("M*") {
            current = regex::Regex::new("M[0123sz]")
                .expect("Always valid")
                .replace_all(current.as_str(), "M*")
                .to_string();
        }
        if filter.contains("B*") {
            current = regex::Regex::new("B[0123]")
                .expect("Always valid")
                .replace_all(current.as_str(), "B*")
                .to_string();
        }
        current
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
