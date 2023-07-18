//!
//! The Vyper compiler wrapper.
//!

pub mod vyper_cache_key;

use std::collections::BTreeMap;
use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;
use std::str::FromStr;

use itertools::Itertools;

use super::cache::Cache;
use super::mode::vyper::Mode as VyperMode;
use super::mode::Mode;
use super::output::build::Build as zkEVMContractBuild;
use super::output::Output;
use super::Compiler;

use self::vyper_cache_key::VyperCacheKey;

///
/// The Vyper compiler wrapper.
///
pub struct VyperCompiler {
    /// The vyper process output cache.
    cache: Cache<VyperCacheKey, compiler_vyper::Project>,
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
    /// A shortcut constructor.
    ///
    pub fn new() -> Self {
        Self {
            cache: Cache::new(),
        }
    }

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
    /// Runs the vyper subprocess and returns the project.
    ///
    fn run_vyper(
        sources: &[(String, String)],
        mode: &VyperMode,
    ) -> anyhow::Result<compiler_vyper::Project> {
        let vyper = Self::get_vyper_by_version(&mode.vyper_version);

        let paths = sources
            .iter()
            .map(|(path, _)| {
                PathBuf::from_str(path).map_err(|error| anyhow::anyhow!("Invalid path: {}", error))
            })
            .collect::<anyhow::Result<Vec<PathBuf>>>()?;

        vyper.batch(&mode.vyper_version, paths, mode.vyper_optimize)
    }

    ///
    /// Computes or loads from the cache vyper project. Updates the cache if needed.
    ///
    fn run_vyper_cached(
        &self,
        test_path: String,
        sources: &[(String, String)],
        mode: &VyperMode,
    ) -> anyhow::Result<compiler_vyper::Project> {
        let cache_key =
            VyperCacheKey::new(test_path, mode.vyper_version.clone(), mode.vyper_optimize);

        if !self.cache.contains(&cache_key) {
            self.cache
                .compute(cache_key.clone(), || Self::run_vyper(sources, mode));
        }

        self.cache.get_cloned(&cache_key)
    }

    ///
    /// Compile the vyper project.
    ///
    fn compile(
        project: compiler_vyper::Project,
        mode: &VyperMode,
        debug_config: Option<compiler_llvm_context::DebugConfig>,
    ) -> anyhow::Result<HashMap<String, zkEVMContractBuild>> {
        let build = project.compile(
            mode.llvm_optimizer_settings.to_owned(),
            true,
            zkevm_assembly::get_encoding_mode(),
            debug_config,
        )?;
        build
            .contracts
            .into_iter()
            .map(|(path, contract)| {
                let assembly = zkevm_assembly::Assembly::from_string(
                    contract.build.assembly_text,
                    contract.build.metadata_hash,
                )
                .expect("Always valid");
                Ok((
                    path,
                    zkEVMContractBuild::new_with_hash(assembly, contract.build.bytecode_hash)?,
                ))
            })
            .collect()
    }

    ///
    /// Get the method identifiers from the solc output.
    ///
    fn get_method_identifiers(
        project: &compiler_vyper::Project,
    ) -> anyhow::Result<BTreeMap<String, BTreeMap<String, u32>>> {
        let mut method_identifiers = BTreeMap::new();
        for (path, contract) in project.contracts.iter() {
            let contract_abi = match contract {
                compiler_vyper::Contract::Vyper(inner) => &inner.abi,
                compiler_vyper::Contract::LLVMIR(_inner) => panic!("Only used in the Vyper CLI"),
                compiler_vyper::Contract::ZKASM(_inner) => panic!("Only used in the Vyper CLI"),
            };
            let mut contract_identifiers = BTreeMap::new();
            for (entry, hash) in contract_abi.iter() {
                let selector = u32::from_str_radix(&hash[2..], compiler_common::BASE_HEXADECIMAL)
                    .map_err(|error| {
                    anyhow::anyhow!("Invalid selector from the Vyper compiler: {}", error)
                })?;
                contract_identifiers.insert(entry.clone(), selector);
            }
            method_identifiers.insert(path.clone(), contract_identifiers);
        }
        Ok(method_identifiers)
    }

    ///
    /// Prints LLL IR if the flag is set.
    ///
    fn dump_lll(
        sources: &[(String, String)],
        debug_config: &compiler_llvm_context::DebugConfig,
        mode: &VyperMode,
    ) -> anyhow::Result<()> {
        let vyper = Self::get_vyper_by_version(&mode.vyper_version);

        let lll = sources
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
    fn modes(&self) -> Vec<Mode> {
        MODES.clone()
    }

    fn compile(
        &self,
        test_path: String,
        sources: Vec<(String, String)>,
        _libraries: BTreeMap<String, BTreeMap<String, String>>,
        mode: &Mode,
        _is_system_mode: bool,
        _is_system_contracts_mode: bool,
        debug_config: Option<compiler_llvm_context::DebugConfig>,
    ) -> anyhow::Result<Output> {
        let mode = VyperMode::unwrap(mode);

        if let Some(ref debug_config) = debug_config {
            Self::dump_lll(&sources, debug_config, mode)?;
        }

        let project = self
            .run_vyper_cached(test_path, &sources, mode)
            .map_err(|error| anyhow::anyhow!("Failed to get vyper project: {}", error))?;

        let method_identifiers = Self::get_method_identifiers(&project)
            .map_err(|error| anyhow::anyhow!("Failed to get method identifiers: {}", error))?;

        let last_contract = sources
            .last()
            .ok_or_else(|| anyhow::anyhow!("Sources is empty"))?
            .0
            .clone();

        let builds = Self::compile(project, mode, debug_config)
            .map_err(|error| anyhow::anyhow!("Failed to compile the contracts: {}", error))?;

        Ok(Output::new(builds, Some(method_identifiers), last_contract))
    }

    fn has_many_contracts(&self) -> bool {
        false
    }
}
