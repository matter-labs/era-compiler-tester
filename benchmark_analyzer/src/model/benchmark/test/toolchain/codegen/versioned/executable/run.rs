//!
//! A run of a test with fixed compiler options (mode).
//!

use serde::Deserialize;
use serde::Serialize;

///
/// A run of a test with fixed compiler options (mode).
///
/// All fields are vectors to allow for multiple measurements with averaging capabilities.
///
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Run {
    /// Contract full size, includes both deploy and runtime code.
    #[serde(default)]
    pub size: Vec<u64>,
    /// Contract runtime code size.
    #[serde(default)]
    pub runtime_size: Vec<u64>,
    /// Number of cycles.
    #[serde(default)]
    pub cycles: Vec<u64>,
    /// Amount of ergs.
    #[serde(default)]
    pub ergs: Vec<u64>,
    /// Amount of EVM gas.
    #[serde(default)]
    pub gas: Vec<u64>,
}

impl Run {
    ///
    /// Average contract size.
    ///
    pub fn average_size(&self) -> u64 {
        if self.size.is_empty() {
            return 0;
        }

        self.size.iter().sum::<u64>() / (self.size.len() as u64)
    }

    ///
    /// Average runtime code size.
    ///
    pub fn average_runtime_size(&self) -> u64 {
        if self.runtime_size.is_empty() {
            return 0;
        }

        self.runtime_size.iter().sum::<u64>() / (self.runtime_size.len() as u64)
    }

    ///
    /// Average number of cycles.
    ///
    pub fn average_cycles(&self) -> u64 {
        if self.cycles.is_empty() {
            return 0;
        }

        self.cycles.iter().sum::<u64>() / (self.cycles.len() as u64)
    }

    ///
    /// Average amount of ergs.
    ///
    pub fn average_ergs(&self) -> u64 {
        if self.ergs.is_empty() {
            return 0;
        }

        self.ergs.iter().sum::<u64>() / (self.ergs.len() as u64)
    }

    ///
    /// Average amount of EVM gas.
    ///
    pub fn average_gas(&self) -> u64 {
        if self.gas.is_empty() {
            return 0;
        }

        self.gas.iter().sum::<u64>() / (self.gas.len() as u64)
    }
}
