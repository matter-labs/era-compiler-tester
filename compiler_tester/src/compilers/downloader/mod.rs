//!
//! The compiler downloader.
//!

pub mod config;
pub mod solc_list;

use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::path::PathBuf;
use std::str::FromStr;

use colored::Colorize;

use self::config::binary::protocol::Protocol;
use self::config::Config;
use self::solc_list::SolcList;

///
/// The compiler downloader.
///
#[derive(Debug, Default)]
pub struct Downloader {
    /// The solc-bin JSON list metadata.
    solc_list: Option<SolcList>,
}

impl Downloader {
    ///
    /// Downloads the compilers described in the config.
    ///
    pub fn download(mut self, config_path: &Path) -> anyhow::Result<Config> {
        let config_file = std::fs::File::open(config_path).map_err(|error| {
            anyhow::anyhow!("Binaries download config opening error: {}", error)
        })?;
        let config_reader = std::io::BufReader::new(config_file);
        let config: Config = serde_json::from_reader(config_reader).map_err(|error| {
            anyhow::anyhow!("Binaries download config parsing error: {}", error)
        })?;

        let platform_directory = config.get_remote_platform_directory()?;

        for (version, binary) in config.binaries.iter() {
            if !binary.is_enabled {
                continue;
            }

            let source_path = binary
                .source
                .replace("${PLATFORM}", platform_directory.as_str());

            let destination_path = PathBuf::from_str(binary.destination.as_str())
                .map_err(|_| anyhow::anyhow!("Binary `{}` destination is invalid", source_path))?;

            let data = match binary.protocol {
                Protocol::File => {
                    if source_path == destination_path.to_string_lossy() {
                        continue;
                    }

                    println!(
                        "     {} binary `{}` => `{}`",
                        "Copying".bright_green().bold(),
                        source_path,
                        binary.destination,
                    );

                    std::fs::copy(source_path.as_str(), binary.destination.as_str())
                        .map_err(|error| anyhow::anyhow!("Binary copying error: {}", error))?;
                    continue;
                }
                Protocol::HTTPS => {
                    if destination_path.exists() {
                        continue;
                    }

                    let source_url =
                        reqwest::Url::from_str(source_path.as_str()).expect("Always valid");
                    println!(
                        " {} binary `{}` => `{}`",
                        "Downloading".bright_green().bold(),
                        source_url,
                        binary.destination,
                    );
                    reqwest::blocking::get(source_url)?.bytes()?
                }
                Protocol::SolcBinList => {
                    if destination_path.exists() {
                        continue;
                    }

                    let solc_list_path = PathBuf::from(source_path.as_str());
                    let solc_list = self.solc_list.get_or_insert_with(|| {
                        SolcList::try_from(solc_list_path.as_path())
                            .expect("solc-bin JSON list downloading error")
                    });
                    if solc_list.releases.is_empty() {
                        return Ok(config);
                    }

                    let source_binary_name =
                        match solc_list.releases.get(version.to_string().as_str()) {
                            Some(source_binary_name) => source_binary_name,
                            None => anyhow::bail!(
                                "Binary for version v{} not found in the solc JSON list",
                                version
                            ),
                        };
                    let mut source_path = solc_list_path;
                    source_path.pop();
                    source_path.push(source_binary_name);

                    let source_url =
                        reqwest::Url::from_str(source_path.to_str().expect("Always valid"))
                            .expect("Always valid");
                    println!(
                        " {} binary `{}` => `{}`",
                        "Downloading".bright_green().bold(),
                        source_url,
                        binary.destination,
                    );
                    reqwest::blocking::get(source_url)?.bytes()?
                }
            };

            let mut destination_folder = destination_path.clone();
            destination_folder.pop();
            std::fs::create_dir_all(destination_folder)?;

            std::fs::write(&destination_path, data)?;
            std::fs::set_permissions(&destination_path, std::fs::Permissions::from_mode(0o755))?;
        }

        Ok(config)
    }
}
