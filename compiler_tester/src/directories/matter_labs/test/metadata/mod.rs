//!
//! The Matter Labs compiler test metadata.
//!

pub mod case;
pub mod evm_contract;

use std::collections::BTreeMap;
use std::str::FromStr;

use crate::environment::Environment;

use self::case::Case;
use self::evm_contract::EVMContract;

///
/// The Matter Labs compiler test metadata.
///
#[derive(Debug, Clone, serde::Deserialize)]
pub struct Metadata {
    /// The test cases.
    pub cases: Vec<Case>,
    /// The mode filter.
    pub modes: Option<Vec<String>>,
    /// The test contracts.
    /// The format is `instance -> path`.
    #[serde(default)]
    pub contracts: BTreeMap<String, String>,
    /// The EVM auxiliary contracts.
    /// The format is `instance -> init code`.
    #[serde(default)]
    pub evm_contracts: BTreeMap<String, EVMContract>,
    /// The test libraries for linking.
    #[serde(default)]
    pub libraries: BTreeMap<String, BTreeMap<String, String>>,
    /// Enable the EraVM extensions.
    #[serde(default)]
    pub enable_eravm_extensions: bool,
    /// The environment to run the tests on.
    pub environment: Option<Environment>,
    /// If the entire test file must be ignored.
    #[serde(default)]
    pub ignore: bool,
    /// The test group.
    pub group: Option<String>,
}

impl FromStr for Metadata {
    type Err = anyhow::Error;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        let json = string
            .lines()
            .filter_map(|line| {
                line.strip_prefix("//!")
                    .or_else(|| line.strip_prefix(";!"))
                    .or_else(|| line.strip_prefix("#!"))
            })
            .collect::<Vec<&str>>()
            .join("");

        serde_json::from_str(json.as_str()).or_else(|_| Ok(serde_json::from_str(string)?))
    }
}
