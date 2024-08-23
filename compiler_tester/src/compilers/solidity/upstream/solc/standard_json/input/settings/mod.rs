//!
//! The `solc --standard-json` input settings.
//!

pub mod debug;
pub mod optimizer;
pub mod selection;

use std::collections::BTreeMap;
use std::collections::BTreeSet;

use serde::Serialize;

use self::debug::Debug;
use self::optimizer::Optimizer;
use self::selection::Selection;

///
/// The `solc --standard-json` input settings.
///
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    /// The target EVM version.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evm_version: Option<era_compiler_common::EVMVersion>,
    /// The linker library addresses.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub libraries: Option<BTreeMap<String, BTreeMap<String, String>>>,
    /// The sorted list of remappings.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remappings: Option<BTreeSet<String>>,
    /// The output selection filters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_selection: Option<Selection>,
    /// Whether to compile via IR. Only for testing with solc >=0.8.13.
    #[serde(
        rename = "viaIR",
        skip_serializing_if = "Option::is_none",
        skip_deserializing
    )]
    pub via_ir: Option<bool>,
    /// The optimizer settings.
    pub optimizer: Optimizer,
    /// The debug settings.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub debug: Option<Debug>,
}

impl Settings {
    ///
    /// A shortcut constructor.
    ///
    pub fn new(
        evm_version: Option<era_compiler_common::EVMVersion>,
        libraries: BTreeMap<String, BTreeMap<String, String>>,
        remappings: Option<BTreeSet<String>>,
        output_selection: Selection,
        via_ir: bool,
        optimizer: Optimizer,
        debug: Option<Debug>,
    ) -> Self {
        Self {
            evm_version,
            libraries: Some(libraries),
            remappings,
            output_selection: Some(output_selection),
            via_ir: if via_ir { Some(true) } else { None },
            optimizer,
            debug,
        }
    }
}
