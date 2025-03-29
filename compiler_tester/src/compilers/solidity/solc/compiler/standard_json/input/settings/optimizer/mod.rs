//!
//! The `solc --standard-json` input settings optimizer.
//!

use serde::Serialize;

///
/// The `solc --standard-json` input settings optimizer.
///
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Optimizer {
    /// Whether the optimizer is enabled.
    pub enabled: bool,
}

impl Optimizer {
    ///
    /// A shortcut constructor.
    ///
    pub fn new(enabled: bool) -> Self {
        Self { enabled }
    }
}
