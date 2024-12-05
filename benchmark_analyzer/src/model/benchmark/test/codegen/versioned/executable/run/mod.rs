//!
//! A run of a test with fixed compilation settins (mode)
//!

use serde::Deserialize;
use serde::Serialize;

///
/// A run of a test with fixed compilation settins (mode).
///
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Run {
    /// The contract size, `Some` for contracts deploys.
    pub size: Option<usize>,
    /// The number of cycles.
    pub cycles: usize,
    /// The amount of ergs.
    pub ergs: u64,
    /// The amount of EVM gas.
    pub gas: u64,
}

impl Run {
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
