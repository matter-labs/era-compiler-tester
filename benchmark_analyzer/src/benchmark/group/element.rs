//!
//! The benchmark element.
//!

use serde::Deserialize;
use serde::Serialize;

///
/// The benchmark element.
///
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Element {
    /// The contract size, `Some` for contracts deploys.
    pub size: Option<usize>,
    /// The number of cycles.
    pub cycles: usize,
}

impl Element {
    ///
    /// A shortcut constructor.
    ///
    pub fn new(size: Option<usize>, cycles: usize) -> Self {
        Self { size, cycles }
    }
}
