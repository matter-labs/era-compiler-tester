//!
//! The compiler downloader config.
//!

pub mod binary;

use std::collections::BTreeMap;
use std::collections::HashMap;

use serde::Deserialize;

use self::binary::Binary;

///
/// The compiler downloader config.
///
#[derive(Debug, Deserialize)]
pub struct Config {
    /// The compiler binaries to download.
    pub binaries: BTreeMap<String, Binary>,
    /// The compiler platform directory names.
    pub platforms: Option<HashMap<String, String>>,
}

impl Config {
    ///
    /// Returns the remote platform directory name for the specified platform.
    ///
    pub fn get_remote_platform_directory(&self) -> anyhow::Result<String> {
        let platforms = match self.platforms {
            Some(ref platform) => platform,
            None => anyhow::bail!("Platforms are not defined"),
        };

        Ok(if cfg!(target_os = "linux") {
            platforms
                .get("linux")
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("Linux platform directory is not defined"))?
        } else if cfg!(target_os = "macos") {
            platforms
                .get("macos")
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("MacOS platform directory is not defined"))?
        } else {
            anyhow::bail!("Unsupported platform!")
        })
    }
}
