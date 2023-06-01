//!
//! The Vyper compiler wrapper.
//!

pub mod cached_project;
pub mod subprocess_mode;

use std::collections::BTreeMap;
use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;
use std::str::FromStr;

use itertools::Itertools;
use sha3::Digest;

use super::build::Build as zkEVMContractBuild;
use super::cache::Cache;
use super::mode::vyper::Mode as VyperMode;
use super::mode::Mode;
use super::Compiler;

use self::cached_project::CachedProject;
use self::subprocess_mode::SubprocessMode;

///
/// The Vyper compiler wrapper.
///
pub struct VyperCompiler {
    /// The name-to-code source files mapping.
    sources: Vec<(String, String)>,
    /// The vyper process output cache.
    cache: Cache<SubprocessMode, anyhow::Result<CachedProject>>,
    /// The compiler debug config.
    debug_config: Option<compiler_llvm_context::DebugConfig>,
}

lazy_static::lazy_static! {
    ///
    /// The Vyper compiler supported modes.
    ///
    static ref MODES: Vec<Mode> = {
        let vyper_versions = VyperCompiler::all_versions().expect("`vyper` versions analysis error");

        compiler_llvm_context::OptimizerSettings::combinations()
            .into_iter()
            .cartesian_product(vyper_versions)
            .cartesian_product(vec![false, true])
            .map(
                |((llvm_optimizer_settings, vyper_version), vyper_optimize)| {
                    VyperMode::new(vyper_version, vyper_optimize, llvm_optimizer_settings).into()
                },
            )
            .collect::<Vec<Mode>>()
    };
}

impl VyperCompiler {
    /// The compiler binaries directory.
    pub const DIRECTORY: &'static str = "vyper-bin/";

    ///
    /// Returns the Vyper compiler instance by version.
    ///
    fn get_vyper_by_version(version: &semver::Version) -> compiler_vyper::VyperCompiler {
        compiler_vyper::VyperCompiler::new(format!("{}/vyper-{}", Self::DIRECTORY, version))
    }

    ///
    /// Returns the downloaded compiler versions.
    ///
    fn all_versions() -> anyhow::Result<Vec<semver::Version>> {
        let mut versions = Vec::new();
        for entry in std::fs::read_dir("./vyper-bin/")? {
            let entry = entry?;
            let path = entry.path();
            let entry_type = entry.file_type().map_err(|error| {
                anyhow::anyhow!(
                    "File `{}` type getting error: {}",
                    path.to_string_lossy(),
                    error
                )
            })?;
            if !entry_type.is_file() {
                anyhow::bail!(
                    "Invalid `vyper` binary file type: {}",
                    path.to_string_lossy()
                );
            }

            let file_name = entry.file_name().to_string_lossy().to_string();
            let version_str = match file_name.strip_prefix("vyper-") {
                Some(version_str) => version_str,
                None => continue,
            };
            let version: semver::Version = match version_str.parse() {
                Ok(version) => version,
                Err(_) => continue,
            };
            versions.push(version);
        }
        Ok(versions)
    }

    ///
    /// Starts building the project into cache if it has not been built yet.
    ///
    fn start_building(&self, mode: &SubprocessMode) {
        if !self.cache.contains(mode) {
            if let Some(waiter) = self.cache.start(mode.clone()) {
                self.cache
                    .finish(mode.clone(), self.get_build(mode), waiter);
            };
        }
    }

    ///
    /// Gets the cached project data from the cache.
    ///
    fn get_build(&self, mode: &SubprocessMode) -> anyhow::Result<CachedProject> {
        let vyper = Self::get_vyper_by_version(&mode.version);

        let paths = self
            .sources
            .iter()
            .map(|(path, _)| {
                PathBuf::from_str(path).map_err(|error| anyhow::anyhow!("Invalid path: {}", error))
            })
            .collect::<anyhow::Result<Vec<PathBuf>>>()?;

        let project = vyper.batch(&mode.version, paths, mode.optimize)?;

        Ok(CachedProject::new(project))
    }

    ///
    /// Prints LLL IR if the flag is set.
    ///
    fn dump_lll(
        &self,
        debug_config: &compiler_llvm_context::DebugConfig,
        mode: &VyperMode,
    ) -> anyhow::Result<()> {
        let vyper = Self::get_vyper_by_version(&mode.vyper_version);

        let lll = self
            .sources
            .iter()
            .map(|(path_str, _)| {
                let path = Path::new(path_str.as_str());
                vyper
                    .lll_debug(path, mode.vyper_optimize)
                    .map(|lll| (path_str.to_string(), lll))
            })
            .collect::<anyhow::Result<Vec<(String, String)>>>()?;

        for (path, lll) in lll.iter() {
            debug_config.dump_lll(path, lll)?;
        }

        Ok(())
    }
}

impl Compiler for VyperCompiler {
    fn new(
        sources: Vec<(String, String)>,
        _libraries: BTreeMap<String, BTreeMap<String, String>>,
        debug_config: Option<compiler_llvm_context::DebugConfig>,
        _is_system_mode: bool,
    ) -> Self {
        Self {
            sources,
            cache: Cache::new(),
            debug_config,
        }
    }

    fn modes() -> Vec<Mode> {
        MODES.clone()
    }

    fn compile(
        &self,
        mode: &Mode,
        _is_system_contract_mode: bool,
    ) -> anyhow::Result<HashMap<String, zkEVMContractBuild>> {
        let mode = VyperMode::unwrap(mode);

        if let Some(ref debug_config) = self.debug_config {
            self.dump_lll(debug_config, mode)?;
        }

        let subprocess_mode = SubprocessMode::new(mode.vyper_version.clone(), mode.vyper_optimize);

        self.start_building(&subprocess_mode);

        let cached_project = {
            self.cache.wait(&subprocess_mode);
            let lock = self.cache.read();
            let cached_project = lock
                .get(&subprocess_mode)
                .expect("Always valid")
                .unwrap_value();
            cached_project
                .as_ref()
                .map_err(|error| anyhow::anyhow!(error.to_string()))?
                .project
                .clone()
        };

        let target_machine =
            compiler_llvm_context::TargetMachine::new(&mode.llvm_optimizer_settings)?;
        let build = cached_project.compile(
            target_machine,
            mode.llvm_optimizer_settings.clone(),
            true,
            self.debug_config.clone(),
        )?;

        let mut forwarder_needed = false;
        let mut builds = build
            .contracts
            .into_iter()
            .map(|(path, contract)| {
                if contract
                    .build
                    .factory_dependencies
                    .contains_key(compiler_vyper::FORWARDER_CONTRACT_HASH.as_str())
                {
                    forwarder_needed = true;
                }
                Ok((
                    path,
                    zkEVMContractBuild::new_with_hash(
                        contract.build.assembly,
                        contract.build.bytecode_hash,
                    )?,
                ))
            })
            .collect::<anyhow::Result<HashMap<String, zkEVMContractBuild>>>()?;

        if forwarder_needed {
            builds.insert(
                compiler_vyper::FORWARDER_CONTRACT_NAME.to_owned(),
                zkEVMContractBuild::new_with_hash(
                    zkevm_assembly::Assembly::from_string(
                        compiler_vyper::FORWARDER_CONTRACT_ASSEMBLY.to_owned(),
                        Some(
                            sha3::Keccak256::digest(
                                compiler_vyper::FORWARDER_CONTRACT_ASSEMBLY.as_bytes(),
                            )
                            .into(),
                        ),
                    )
                    .map_err(|error| anyhow::anyhow!("Vyper forwarder assembly: {}", error))?,
                    compiler_vyper::FORWARDER_CONTRACT_HASH.clone(),
                )
                .map_err(|error| anyhow::anyhow!("Vyper forwarder: {}", error))?,
            );
        }

        Ok(builds)
    }

    fn selector(
        &self,
        mode: &Mode,
        contract_path: &str,
        entry: &str,
        _is_system_contract_mode: bool,
    ) -> anyhow::Result<u32> {
        let mode = VyperMode::unwrap(mode);

        let subprocess_mode = SubprocessMode::new(mode.vyper_version.clone(), mode.vyper_optimize);

        self.start_building(&subprocess_mode);

        self.cache.wait(&subprocess_mode);
        let lock = self.cache.read();
        let cached_project = lock
            .get(&subprocess_mode)
            .expect("Always valid")
            .unwrap_value();

        let cached_project = cached_project
            .as_ref()
            .map_err(|error| anyhow::anyhow!(error.to_string()))?;

        let contract_identifiers = match cached_project
            .project
            .contracts
            .get(contract_path)
            .ok_or_else(|| anyhow::anyhow!("Contract {} not found", contract_path))?
        {
            compiler_vyper::Contract::Vyper(inner) => &inner.abi,
            compiler_vyper::Contract::LLVMIR(_inner) => panic!("Only used in the Vyper CLI"),
            compiler_vyper::Contract::ZKASM(_inner) => panic!("Only used in the Vyper CLI"),
        };

        contract_identifiers
            .iter()
            .find_map(|(name, hash)| {
                if name.starts_with(entry) {
                    Some(
                        u32::from_str_radix(&hash[2..], compiler_common::BASE_HEXADECIMAL).map_err(
                            |error| {
                                anyhow::anyhow!(
                                    "Invalid selector from the Vyper compiler: {}",
                                    error
                                )
                            },
                        ),
                    )
                } else {
                    None
                }
            })
            .ok_or_else(|| anyhow::anyhow!("Hash of the method `{}` not found", entry))?
    }

    fn last_contract(
        &self,
        _mode: &Mode,
        _is_system_contract_mode: bool,
    ) -> anyhow::Result<String> {
        Ok(self
            .sources
            .last()
            .ok_or_else(|| anyhow::anyhow!("Sources is empty"))?
            .0
            .clone())
    }

    fn has_many_contracts() -> bool {
        false
    }

    fn check_pragmas(&self, mode: &Mode) -> bool {
        let mode = VyperMode::unwrap(mode);

        self.sources.iter().all(|(_, source_code)| {
            match source_code.lines().find_map(|line| {
                let mut split = line.split_whitespace();
                if let (Some("#"), Some("@version"), Some(version)) =
                    (split.next(), split.next(), split.next())
                {
                    semver::VersionReq::parse(version).ok()
                } else {
                    None
                }
            }) {
                Some(pragma_version_req) => pragma_version_req.matches(&mode.vyper_version),
                None => true,
            }
        })
    }

    fn check_ethereum_tests_params(_mode: &Mode, _params: &solidity_adapter::Params) -> bool {
        true
    }
}
