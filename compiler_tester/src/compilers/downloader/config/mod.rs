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

        let platform = if cfg!(target_arch = "x86_64") {
            if cfg!(target_os = "linux") {
                "linux-amd64"
            } else if cfg!(target_os = "macos") {
                "macos-amd64"
            } else {
                anyhow::bail!("This platform is not supported in `solc`!");
            }
        } else if cfg!(target_arch = "aarch64") {
            if cfg!(target_os = "linux") {
                "linux-arm64"
            } else if cfg!(target_os = "macos") {
                "macos-arm64"
            } else {
                anyhow::bail!("This platform is not supported in `solc`!");
            }
        } else {
            anyhow::bail!("This platform is not supported in `solc`!");
        };

        platforms
            .get(platform)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Directory for platform `{}` is not defined", platform))
    }
}
