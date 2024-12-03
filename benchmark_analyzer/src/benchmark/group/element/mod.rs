//!
//! The benchmark element.
//!

pub mod input;
pub mod metadata;
pub mod selector;

use serde::Deserialize;
use serde::Serialize;

use metadata::Metadata;

///
/// The benchmark element.
///
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Element {
    /// Associated metadata.
    pub metadata: Metadata,
    /// The contract size, `Some` for contracts deploys.
    pub size: Option<usize>,
    /// The number of cycles.
    pub cycles: usize,
    /// The amount of ergs.
    pub ergs: u64,
    /// The amount of EVM gas.
    pub gas: u64,
}

impl Element {
    ///
    /// A shortcut constructor.
    ///
    pub fn new(
        metadata: Metadata,
        size: Option<usize>,
        cycles: usize,
        ergs: u64,
        gas: u64,
    ) -> Self {
        Self {
            metadata,
            size,
            cycles,
            ergs,
            gas,
        }
    }
}
