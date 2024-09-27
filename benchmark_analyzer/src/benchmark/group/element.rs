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
    /// The number of ergs.
    pub ergs: u64,
    /// The number of EVM gas.
    pub gas: u64,
}

impl Element {
    ///
    /// A shortcut constructor.
    ///
    pub fn new(size: Option<usize>, cycles: usize, ergs: u64, gas: u64) -> Self {
        Self {
            size,
            cycles,
            ergs,
            gas,
        }
    }
}
