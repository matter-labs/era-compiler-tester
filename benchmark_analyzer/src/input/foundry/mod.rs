//!
//! Foundry benchmark report format.
//!

pub mod contract;

use std::path::Path;

use self::contract::ContractReport;

///
/// Foundry benchmark report format.
///
#[derive(Debug, serde::Deserialize)]
pub struct FoundryReport {
    /// The original report.
    pub data: Vec<ContractReport>,

    /// Project identifier.
    /// Must be added to the original Foundry report.
    pub project: String,
    /// Optional toolchain identifier.
    /// Must be added to the original Foundry report.
    pub toolchain: String,
}

impl TryFrom<&Path> for FoundryReport {
    type Error = anyhow::Error;

    fn try_from(path: &Path) -> Result<Self, Self::Error> {
        let text = std::fs::read_to_string(path)
            .map_err(|error| anyhow::anyhow!("Foundry report file {path:?} reading: {error}"))?;
        let json: Self = serde_json::from_str(text.as_str())
            .map_err(|error| anyhow::anyhow!("Foundry report file {path:?} parsing: {error}"))?;
        Ok(json)
    }
}
