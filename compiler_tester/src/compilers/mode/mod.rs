//!
//! The compiler mode.
//!

pub mod llvm_options;

use std::collections::HashSet;

use crate::compilers::eravm::mode::Mode as EraVMMode;
use crate::compilers::llvm::mode::Mode as LLVMMode;
use crate::compilers::solidity::mode::Mode as SolidityMode;
use crate::compilers::solidity::upstream::mode::Mode as SolidityUpstreamMode;
use crate::compilers::vyper::mode::Mode as VyperMode;
use crate::compilers::yul::mode::Mode as YulMode;

///
/// The compiler mode.
///
#[derive(Debug, Clone)]
#[allow(clippy::upper_case_acronyms)]
pub enum Mode {
    /// The `Solidity` mode.
    Solidity(SolidityMode),
    /// The `Solidity` upstream mode.
    SolidityUpstream(SolidityUpstreamMode),
    /// The `Yul` mode.
    Yul(YulMode),
    /// The `Vyper` mode.
    Vyper(VyperMode),
    /// The `LLVM` mode.
    LLVM(LLVMMode),
    /// The `EraVM` mode.
    EraVM(EraVMMode),
}

impl Mode {
    ///
    /// Sets the system mode if applicable.
    ///
    pub fn set_system_mode(&mut self, value: bool) {
        match self {
            Self::Solidity(mode) => mode.is_system_mode = value,
            Self::Yul(mode) => mode.is_system_mode = value,
            _ => {}
        }
    }

    ///
    /// Checks if the mode is compatible with the filters.
    ///
    pub fn check_filters(&self, filters: &HashSet<String>) -> bool {
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
            Mode::SolidityUpstream(mode) => &mode.solc_version,
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
            Mode::SolidityUpstream(mode) => mode.check_pragmas(sources),
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
            Mode::SolidityUpstream(mode) => mode.check_ethereum_tests_params(params),
            _ => true,
        }
    }

    ///
    /// Returns the LLVM optimizer settings.
    ///
    pub fn llvm_optimizer_settings(&self) -> Option<&era_compiler_llvm_context::OptimizerSettings> {
        match self {
            Mode::Solidity(mode) => Some(&mode.llvm_optimizer_settings),
            Mode::SolidityUpstream(_mode) => None,
            Mode::Yul(mode) => Some(&mode.llvm_optimizer_settings),
            Mode::Vyper(mode) => Some(&mode.llvm_optimizer_settings),
            Mode::LLVM(mode) => Some(&mode.llvm_optimizer_settings),
            Mode::EraVM(_mode) => None,
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

        if filter.starts_with('^') {
            match self {
                Self::Solidity(_) | Self::SolidityUpstream(_) | Self::Vyper(_) => {
                    current = regex::Regex::new("[+]")
                        .expect("Always valid")
                        .replace_all(current.as_str(), "^")
                        .to_string();
                }
                Self::Yul(_) | Self::LLVM(_) => {
                    current = regex::Regex::new(".*M")
                        .expect("Always valid")
                        .replace_all(current.as_str(), "^M")
                        .to_string();
                }
                Self::EraVM(_) => {}
            }
        }

        current
    }
}

impl From<SolidityMode> for Mode {
    fn from(inner: SolidityMode) -> Self {
        Self::Solidity(inner)
    }
}

impl From<SolidityUpstreamMode> for Mode {
    fn from(inner: SolidityUpstreamMode) -> Self {
        Self::SolidityUpstream(inner)
    }
}

impl From<YulMode> for Mode {
    fn from(inner: YulMode) -> Self {
        Self::Yul(inner)
    }
}

impl From<VyperMode> for Mode {
    fn from(inner: VyperMode) -> Self {
        Self::Vyper(inner)
    }
}

impl From<LLVMMode> for Mode {
    fn from(inner: LLVMMode) -> Self {
        Self::LLVM(inner)
    }
}

impl From<EraVMMode> for Mode {
    fn from(inner: EraVMMode) -> Self {
        Self::EraVM(inner)
    }
}

impl std::fmt::Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Solidity(inner) => write!(f, "{inner}"),
            Self::SolidityUpstream(inner) => write!(f, "{inner}"),
            Self::Yul(inner) => write!(f, "{inner}"),
            Self::Vyper(inner) => write!(f, "{inner}"),
            Self::LLVM(inner) => write!(f, "{inner}"),
            Self::EraVM(inner) => write!(f, "{inner}"),
        }
    }
}
