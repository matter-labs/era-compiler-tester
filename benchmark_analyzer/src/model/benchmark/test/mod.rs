//!
//! The benchmark group representation.
//!

pub mod codegen;
pub mod input;
pub mod metadata;
pub mod selector;

use std::collections::BTreeMap;

use codegen::CodegenGroup;
use metadata::Metadata;
use serde::Deserialize;
use serde::Serialize;

///
/// The codegen associated with a test definition.
///
pub type Codegen = String;

///
/// The benchmark group representation.
///
#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct Test {
    /// Metadata for this test.
    #[serde(default)]
    pub metadata: Metadata,
    /// Versions.
    pub codegen_groups: BTreeMap<Codegen, CodegenGroup>,
}

impl Test {
    ///
    /// Creates a new test with provided metadata.
    ///
    pub fn new(metadata: Metadata) -> Self {
        Self {
            codegen_groups: Default::default(),
            metadata,
        }
    }
}
