//!
//! The `solc --standard-json` expected output selection flag.
//!

use serde::Serialize;

///
/// The `solc --standard-json` expected output selection flag.
///
#[derive(Debug, Serialize, Clone, Copy, PartialEq, Eq, Hash)]
#[allow(non_camel_case_types)]
#[allow(clippy::upper_case_acronyms)]
pub enum Flag {
    /// The combined bytecode.
    #[serde(rename = "evm.bytecode")]
    Bytecode,
    /// The function signature hashes JSON.
    #[serde(rename = "evm.methodIdentifiers")]
    MethodIdentifiers,
    /// The AST JSON.
    #[serde(rename = "ast")]
    AST,
    /// The Yul IR.
    #[serde(rename = "irOptimized")]
    Yul,
    /// The EVM legacy assembly JSON.
    #[serde(rename = "evm.legacyAssembly")]
    EVMLA,
}

impl From<era_compiler_solidity::SolcPipeline> for Flag {
    fn from(pipeline: era_compiler_solidity::SolcPipeline) -> Self {
        match pipeline {
            era_compiler_solidity::SolcPipeline::Yul => Self::Yul,
            era_compiler_solidity::SolcPipeline::EVMLA => Self::EVMLA,
        }
    }
}

impl std::fmt::Display for Flag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Bytecode => write!(f, "evm.bytecode"),
            Self::MethodIdentifiers => write!(f, "evm.methodIdentifiers"),
            Self::AST => write!(f, "ast"),
            Self::Yul => write!(f, "irOptimized"),
            Self::EVMLA => write!(f, "evm.legacyAssembly"),
        }
    }
}
