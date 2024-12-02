//!
//! Information associated with the benchmark element.
//!

use serde::Deserialize;
use serde::Serialize;

use crate::benchmark::group::element::selector::Selector;

///
/// Encoded compiler mode. In future, it can be expanded into a structured type
/// shared between crates `benchmark_analyzer` and `compiler_tester`.
///
pub type Mode = String;

///
/// Information associated with the benchmark element.
///
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metadata {
    /// Test selector.
    pub selector: Selector,
    /// Compiler mode.
    pub mode: Option<Mode>,
    /// Test group
    pub group: String,
}
