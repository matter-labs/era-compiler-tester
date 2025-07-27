//!
//! Foundry contract benchmark report.
//!

pub mod deployment;
pub mod function;

use std::collections::BTreeMap;

use self::deployment::Deployment;
use self::function::FunctionReport;

///
/// Foundry contract benchmark report.
///
#[derive(Debug, serde::Deserialize)]
pub struct ContractReport {
    /// Contract identifier.
    pub contract: String,
    /// Deployment measurements.
    pub deployment: Deployment,
    /// Per-function measurements.
    pub functions: BTreeMap<String, FunctionReport>,
}
