//!
//! The `solc --standard-json` input settings optimizer.
//!

use serde::Serialize;

///
/// The `solc --standard-json` input settings optimizer.
///
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DebugOpt {
    /// Whether the optimizer is enabled.
    pub revert_strings: String,
}

impl DebugOpt {
    ///
    /// A shortcut constructor.
    ///
    pub fn new(revert_strings: String) -> Self {
        Self { revert_strings }
    }
}
