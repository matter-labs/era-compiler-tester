//!
//! A run of a test with fixed compiler options (mode).
//!

use crate::util::is_zero;
use serde::Deserialize;
use serde::Serialize;

///
/// A run of a test with fixed compiler options (mode).
///
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Run {
    /// The contract size, `Some` for contracts deploys.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
    /// The number of cycles.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub cycles: u64,
    /// The amount of ergs.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub ergs: u64,
    /// The amount of EVM gas.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub gas: u64,
}

impl Run {
    ///
    /// A shortcut constructor.
    ///
    pub fn new(size: Option<u64>, cycles: u64, ergs: u64, gas: u64) -> Self {
        Self {
            size,
            cycles,
            ergs,
            gas,
        }
    }
}
