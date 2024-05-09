//!
//! The `solc --standard-json` output contract EVM data.
//!

pub mod bytecode;

use std::collections::BTreeMap;

use serde::Deserialize;

use self::bytecode::Bytecode;

///
/// The `solc --standard-json` output contract EVM data.
///
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct EVM {
    /// The contract bytecode.
    /// Is reset by that of EraVM before yielding the compiled project artifacts.
    pub bytecode: Option<Bytecode>,
    /// The contract function signatures.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub method_identifiers: Option<BTreeMap<String, String>>,
}
