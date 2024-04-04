//!
//! The Solidity compiler JSON list metadata.
//!

use std::collections::BTreeMap;
use std::path::Path;
use std::str::FromStr;

use colored::Colorize;
use serde::Deserialize;

///
/// The Solidity compiler JSON list metadata.
///
#[derive(Debug, Deserialize)]
pub struct SolcList {
    /// The collection of compiler releases.
    pub releases: BTreeMap<String, String>,
}

impl TryFrom<&Path> for SolcList {
    type Error = anyhow::Error;

    fn try_from(path: &Path) -> Result<Self, Self::Error> {
        let url =
            reqwest::Url::from_str(path.to_str().expect("Always valid")).expect("Always valid");
        println!(
            " {} solc-bin JSON `{}`",
            "Downloading".bright_green().bold(),
            url
        );
        let list: SolcList = reqwest::blocking::get(url)?.json()?;
        Ok(list)
    }
}
