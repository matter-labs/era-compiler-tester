//!
//! Foundry benchmark report format.
//!

pub mod contract_report;

use std::path::Path;

use self::contract_report::ContractReport;

///
/// Foundry benchmark report format.
///
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct FoundryReport(pub Vec<ContractReport>);

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
