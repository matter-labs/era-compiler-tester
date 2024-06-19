//!
//! The Vyper compiler.
//!

pub mod cache_key;
pub mod mode;

use std::collections::BTreeMap;
use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;
use std::str::FromStr;

use itertools::Itertools;

use crate::compilers::cache::Cache;
use crate::compilers::mode::Mode;
use crate::compilers::Compiler;
use crate::vm::eravm::input::build::Build as EraVMBuild;
use crate::vm::eravm::input::Input as EraVMInput;

use self::cache_key::CacheKey;
use self::mode::Mode as VyperMode;

///
/// The Vyper compiler.
///
pub struct VyperCompiler {
    /// The vyper process output cache.
    cache: Cache<CacheKey, era_compiler_vyper::Project>,
}

lazy_static::lazy_static! {
    ///
    /// All supported modes.
    ///
    static ref MODES: Vec<Mode> = {
        let vyper_versions = VyperCompiler::all_versions().expect("`vyper` versions analysis error");

        era_compiler_llvm_context::OptimizerSettings::combinations()
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

impl Default for VyperCompiler {
    fn default() -> Self {
        Self::new()
    }
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
    /// Returns the Vyper executable by its version.
    ///
    fn executable(version: &semver::Version) -> anyhow::Result<era_compiler_vyper::VyperCompiler> {
        era_compiler_vyper::VyperCompiler::new(
            format!("{}/vyper-{}", Self::DIRECTORY, version).as_str(),
        )
    }

    ///
    /// Returns the downloaded compiler versions.
    ///
    fn all_versions() -> anyhow::Result<Vec<semver::Version>> {
        let mut versions = Vec::new();
        for entry in std::fs::read_dir(Self::DIRECTORY)? {
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
    /// Runs the `vyper` subprocess and returns the project.
    ///
    fn get_project(
        sources: Vec<(String, String)>,
        mode: &VyperMode,
    ) -> anyhow::Result<era_compiler_vyper::Project> {
        let vyper = Self::executable(&mode.vyper_version)?;

        let paths = sources
            .into_iter()
            .map(|(path, _)| {
                PathBuf::from_str(path.as_str()).map_err(|error| {
                    anyhow::anyhow!("Invalid source code path `{}`: {}", path, error)
                })
            })
            .collect::<anyhow::Result<Vec<PathBuf>>>()?;

        let evm_version = if mode.vyper_version == semver::Version::new(0, 3, 10) {
            Some(era_compiler_common::EVMVersion::Cancun)
        } else {
            None
        };

        vyper.batch(&mode.vyper_version, paths, evm_version, mode.vyper_optimize)
    }

    ///
    /// Evaluates the Vyper project or loads it from the cache.
    ///
    fn get_project_cached(
        &self,
        test_path: String,
        sources: Vec<(String, String)>,
        mode: &VyperMode,
    ) -> anyhow::Result<era_compiler_vyper::Project> {
        let cache_key = CacheKey::new(test_path, mode.vyper_version.clone(), mode.vyper_optimize);

        if !self.cache.contains(&cache_key) {
            self.cache
                .evaluate(cache_key.clone(), || Self::get_project(sources, mode));
        }

        self.cache.get_cloned(&cache_key)
    }

    ///
    /// Get the method identifiers from the `vyper` output.
    ///
    fn get_method_identifiers(
        project: &era_compiler_vyper::Project,
    ) -> anyhow::Result<BTreeMap<String, BTreeMap<String, u32>>> {
        let mut method_identifiers = BTreeMap::new();
        for (path, contract) in project.contracts.iter() {
            let contract_abi = match contract {
                era_compiler_vyper::Contract::Vyper(inner) => &inner.abi,
                _ => unreachable!("Invalid contract type"),
            };
            let mut contract_identifiers = BTreeMap::new();
            for (entry, hash) in contract_abi.iter() {
                let selector =
                    u32::from_str_radix(&hash[2..], era_compiler_common::BASE_HEXADECIMAL)
                        .map_err(|error| {
                            anyhow::anyhow!(
                                "Invalid selector `{}` received from the Vyper compiler: {}",
                                hash,
                                error
                            )
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
        debug_config: &era_compiler_llvm_context::DebugConfig,
        mode: &VyperMode,
    ) -> anyhow::Result<()> {
        let vyper = Self::executable(&mode.vyper_version)?;

        let evm_version = if mode.vyper_version == semver::Version::new(0, 3, 10) {
            Some(era_compiler_common::EVMVersion::Cancun)
        } else {
            None
        };

        let lll = sources
            .iter()
            .map(|(path_str, _)| {
                let path = Path::new(path_str.as_str());
                vyper
                    .lll_debug(path, evm_version, mode.vyper_optimize)
                    .map(|lll| (path_str.to_string(), lll))
            })
            .collect::<anyhow::Result<Vec<(String, String)>>>()?;

        for (path, lll) in lll.iter() {
            debug_config.dump_lll(path, None, lll)?;
        }

        Ok(())
    }
}

impl Compiler for VyperCompiler {
    fn compile_for_eravm(
        &self,
        test_path: String,
        sources: Vec<(String, String)>,
        _libraries: BTreeMap<String, BTreeMap<String, String>>,
        mode: &Mode,
        llvm_options: Vec<String>,
        debug_config: Option<era_compiler_llvm_context::DebugConfig>,
    ) -> anyhow::Result<EraVMInput> {
        let mode = VyperMode::unwrap(mode);

        if let Some(ref debug_config) = debug_config {
            Self::dump_lll(&sources, debug_config, mode)?;
        }

        let last_contract = sources
            .last()
            .ok_or_else(|| anyhow::anyhow!("The Vyper sources are empty"))?
            .0
            .clone();

        let project = self
            .get_project_cached(test_path, sources, mode)
            .map_err(|error| anyhow::anyhow!("Failed to get vyper project: {}", error))?;

        let method_identifiers = Self::get_method_identifiers(&project)
            .map_err(|error| anyhow::anyhow!("Failed to get method identifiers: {}", error))?;

        let build = project.compile(
            None,
            true,
            mode.llvm_optimizer_settings.to_owned(),
            llvm_options,
            true,
            zkevm_assembly::get_encoding_mode(),
            vec![],
            debug_config,
        )?;

        let builds = build
            .contracts
            .into_iter()
            .map(|(path, contract)| {
                zkevm_assembly::Assembly::from_string(
                    contract.build.assembly.expect("Always exists"),
                    contract.build.metadata_hash,
                )
                .map_err(anyhow::Error::new)
                .and_then(|assembly| {
                    EraVMBuild::new_with_hash(assembly, contract.build.bytecode_hash)
                })
                .map(|build| (path, build))
            })
            .collect::<anyhow::Result<HashMap<String, EraVMBuild>>>()?;

        Ok(EraVMInput::new(
            builds,
            Some(method_identifiers),
            last_contract,
        ))
    }

    fn compile_for_evm(
        &self,
        _test_path: String,
        _sources: Vec<(String, String)>,
        _libraries: BTreeMap<String, BTreeMap<String, String>>,
        _mode: &Mode,
        _llvm_options: Vec<String>,
        _debug_config: Option<era_compiler_llvm_context::DebugConfig>,
    ) -> anyhow::Result<crate::vm::evm::input::Input> {
        todo!()
    }

    fn all_modes(&self) -> Vec<Mode> {
        MODES.clone()
    }

    fn allows_multi_contract_files(&self) -> bool {
        false
    }
}
