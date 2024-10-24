//!
//! The `solc --standard-json` output file selection.
//!

pub mod flag;

use std::collections::HashSet;

use serde::Serialize;

use self::flag::Flag as SelectionFlag;

///
/// The `solc --standard-json` output file selection.
///
#[derive(Debug, Default, Serialize)]
pub struct File {
    /// The per-file output selections.
    #[serde(rename = "", skip_serializing_if = "Option::is_none")]
    pub per_file: Option<HashSet<SelectionFlag>>,
    /// The per-contract output selections.
    #[serde(rename = "*", skip_serializing_if = "Option::is_none")]
    pub per_contract: Option<HashSet<SelectionFlag>>,
}

impl File {
    ///
    /// Creates the selection required by EVM compilation process.
    ///
    pub fn new_required(codegen: era_compiler_solidity::SolcCodegen) -> Self {
        Self {
            per_file: Some(HashSet::from_iter([SelectionFlag::AST])),
            per_contract: Some(HashSet::from_iter([
                SelectionFlag::Bytecode,
                SelectionFlag::MethodIdentifiers,
                SelectionFlag::from(codegen),
            ])),
        }
    }
}
