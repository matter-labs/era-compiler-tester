//!
//! Benchmark input format.
//!

pub mod format;
pub mod foundry_gas;
pub mod foundry_size;

use std::path::Path;

use self::foundry_gas::FoundryGasReport;
use self::foundry_size::FoundrySizeReport;

///
/// Benchmark input format.
///
#[derive(Debug, serde::Deserialize)]
pub struct Input {
    /// The original report.
    pub data: Report,

    /// Project identifier.
    /// Must be added to the original Foundry report.
    pub project: String,
    /// Optional toolchain identifier.
    /// Must be added to the original Foundry report.
    pub toolchain: String,
}

impl Input {
    ///
    /// Create a new input report.
    ///
    pub fn new(data: Report, project: String, toolchain: String) -> Self {
        Self {
            data,
            project,
            toolchain,
        }
    }
}

///
/// Enum representing various benchmark formats from tooling.
///
#[derive(Debug, serde::Deserialize)]
#[serde(untagged)]
pub enum Report {
    /// Foundry gas report.
    FoundryGas(FoundryGasReport),
    /// Foundry size report.
    FoundrySize(FoundrySizeReport),
}

impl From<FoundryGasReport> for Report {
    fn from(foundry_gas_report: FoundryGasReport) -> Self {
        Report::FoundryGas(foundry_gas_report)
    }
}

impl From<FoundrySizeReport> for Report {
    fn from(foundry_size_report: FoundrySizeReport) -> Self {
        Report::FoundrySize(foundry_size_report)
    }
}

impl TryFrom<&Path> for Input {
    type Error = anyhow::Error;

    fn try_from(path: &Path) -> Result<Self, Self::Error> {
        let text = std::fs::read_to_string(path)
            .map_err(|error| anyhow::anyhow!("Foundry report file {path:?} reading: {error}"))?;
        let json: Self = serde_json::from_str(text.as_str())
            .map_err(|error| anyhow::anyhow!("Foundry report file {path:?} parsing: {error}"))?;
        Ok(json)
    }
}
