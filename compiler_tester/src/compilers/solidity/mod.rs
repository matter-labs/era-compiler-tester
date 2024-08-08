//!
//! The Solidity compiler.
//!

pub mod cache_key;
pub mod mode;
pub mod upstream;

use std::collections::BTreeMap;
use std::collections::HashMap;
use std::path::Path;

use itertools::Itertools;

use era_compiler_solidity::CollectableError;

use crate::compilers::cache::Cache;
use crate::compilers::mode::Mode;
use crate::compilers::Compiler;
use crate::vm::eravm::input::Input as EraVMInput;
use crate::vm::evm::input::build::Build as EVMBuild;
use crate::vm::evm::input::Input as EVMInput;

use self::cache_key::CacheKey;
use self::mode::Mode as SolidityMode;

///
/// The Solidity compiler.
///
pub struct SolidityCompiler {
    /// The `solc` process output cache.
    cache: Cache<CacheKey, era_compiler_solidity::SolcStandardJsonOutput>,
}

lazy_static::lazy_static! {
    ///
    /// All supported modes.
    ///
    /// All compilers must be downloaded before initialization.
    ///
    static ref MODES: Vec<Mode> = {
        let mut solc_pipeline_versions = Vec::new();
        for (pipeline, optimize, via_ir) in [
            (era_compiler_solidity::SolcPipeline::EVMLA, false, false),
            (era_compiler_solidity::SolcPipeline::EVMLA, true, false),
            (era_compiler_solidity::SolcPipeline::EVMLA, true, true),
            (era_compiler_solidity::SolcPipeline::Yul, false, true),
            (era_compiler_solidity::SolcPipeline::Yul, true, true),
        ] {
            for version in SolidityCompiler::all_versions(pipeline, via_ir).expect("`solc` versions analysis error") {
                solc_pipeline_versions.push((pipeline, optimize, via_ir, version));
            }
        }

        era_compiler_llvm_context::OptimizerSettings::combinations()
            .into_iter()
            .cartesian_product(solc_pipeline_versions)
            .map(
                |(mut llvm_optimizer_settings, (pipeline, optimize, via_ir, version))| {
                    llvm_optimizer_settings.enable_fallback_to_size();
                    SolidityMode::new(
                        version,
                        pipeline,
                        via_ir,
                        optimize,
                        llvm_optimizer_settings,
                        false,
                        false,
                    )
                    .into()
                },
            )
            .collect::<Vec<Mode>>()
    };
}

impl Default for SolidityCompiler {
    fn default() -> Self {
        Self::new()
    }
}

impl SolidityCompiler {
    /// The compiler binaries directory.
    const DIRECTORY: &'static str = "solc-bin/";

    /// The solc allow paths argument value.
    const SOLC_ALLOW_PATHS: &'static str = "tests";

    ///
    /// A shortcut constructor.
    ///
    pub fn new() -> Self {
        Self {
            cache: Cache::new(),
        }
    }

    ///
    /// Returns the `solc` executable by its version.
    ///
    pub fn executable(
        version: &semver::Version,
    ) -> anyhow::Result<era_compiler_solidity::SolcCompiler> {
        era_compiler_solidity::SolcCompiler::new(
            format!("{}/solc-{}", Self::DIRECTORY, version).as_str(),
        )
    }

    ///
    /// Returns the `solc` executable used to compile system contracts.
    ///
    pub fn system_contract_executable() -> anyhow::Result<era_compiler_solidity::SolcCompiler> {
        era_compiler_solidity::SolcCompiler::new(
            format!("{}/solc-system-contracts", Self::DIRECTORY).as_str(),
        )
    }

    ///
    /// Returns the compiler versions downloaded for the specified compilation pipeline.
    ///
    pub fn all_versions(
        pipeline: era_compiler_solidity::SolcPipeline,
        via_ir: bool,
    ) -> anyhow::Result<Vec<semver::Version>> {
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
                    "Invalid `solc` binary file type: {}",
                    path.to_string_lossy()
                );
            }

            let file_name = entry.file_name().to_string_lossy().to_string();
            let version_str = match file_name.strip_prefix("solc-") {
                Some(version_str) => version_str,
                None => continue,
            };
            let version: semver::Version = match version_str.parse() {
                Ok(version) => version,
                Err(_) => continue,
            };
            if era_compiler_solidity::SolcPipeline::Yul == pipeline
                && version < era_compiler_solidity::SolcCompiler::FIRST_YUL_VERSION
            {
                continue;
            }
            if era_compiler_solidity::SolcPipeline::EVMLA == pipeline
                && via_ir
                && version < era_compiler_solidity::SolcCompiler::FIRST_VIA_IR_VERSION
            {
                continue;
            }

            versions.push(version);
        }
        Ok(versions)
    }

    ///
    /// Runs the solc subprocess and returns the output.
    ///
    fn standard_json_output(
        sources: &[(String, String)],
        libraries: &BTreeMap<String, BTreeMap<String, String>>,
        mode: &SolidityMode,
    ) -> anyhow::Result<era_compiler_solidity::SolcStandardJsonOutput> {
        let solc_compiler = if mode.is_system_contracts_mode {
            Self::system_contract_executable()
        } else {
            Self::executable(&mode.solc_version)
        }?;

        let mut output_selection =
            era_compiler_solidity::SolcStandardJsonInputSettingsSelection::new_required(Some(
                mode.solc_pipeline,
            ));
        output_selection.extend_with_eravm_assembly();

        let optimizer = era_compiler_solidity::SolcStandardJsonInputSettingsOptimizer::new(
            mode.solc_optimize,
            None,
            &mode.solc_version,
            false,
        );

        let evm_version = if mode.solc_version >= semver::Version::new(0, 8, 24)
        /* TODO */
        {
            Some(era_compiler_common::EVMVersion::Cancun)
        } else {
            None
        };

        let mut solc_input =
            era_compiler_solidity::SolcStandardJsonInput::try_from_solidity_sources(
                evm_version,
                sources.iter().cloned().collect(),
                libraries.clone(),
                None,
                output_selection,
                optimizer,
                None,
                mode.solc_pipeline == era_compiler_solidity::SolcPipeline::EVMLA,
                mode.via_ir,
                mode.enable_eravm_extensions,
                false,
                vec![],
                vec![era_compiler_solidity::MessageType::SendTransfer],
                vec![],
            )
            .map_err(|error| anyhow::anyhow!("Solidity standard JSON I/O error: {}", error))?;

        let allow_paths = Path::new(Self::SOLC_ALLOW_PATHS)
            .canonicalize()
            .expect("Always valid")
            .to_string_lossy()
            .to_string();

        solc_compiler.standard_json(
            &mut solc_input,
            Some(mode.solc_pipeline),
            &mut vec![],
            None,
            vec![],
            Some(allow_paths),
        )
    }

    ///
    /// Evaluates the standard JSON output or loads it from the cache.
    ///
    fn standard_json_output_cached(
        &self,
        test_path: String,
        sources: &[(String, String)],
        libraries: &BTreeMap<String, BTreeMap<String, String>>,
        mode: &SolidityMode,
    ) -> anyhow::Result<era_compiler_solidity::SolcStandardJsonOutput> {
        let cache_key = CacheKey::new(
            test_path,
            mode.solc_version.clone(),
            mode.solc_pipeline,
            mode.via_ir,
            mode.solc_optimize,
        );

        if !self.cache.contains(&cache_key) {
            self.cache.evaluate(cache_key.clone(), || {
                Self::standard_json_output(sources, libraries, mode)
            });
        }

        self.cache.get_cloned(&cache_key)
    }

    ///
    /// Get the method identifiers from the solc output.
    ///
    fn get_method_identifiers(
        solc_output: &era_compiler_solidity::SolcStandardJsonOutput,
    ) -> anyhow::Result<BTreeMap<String, BTreeMap<String, u32>>> {
        let files = solc_output
            .contracts
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Solidity contracts not found in the output"))?;

        let mut method_identifiers = BTreeMap::new();
        for (path, contracts) in files.iter() {
            for (name, contract) in contracts.iter() {
                let mut contract_identifiers = BTreeMap::new();
                for (entry, selector) in contract
                    .evm
                    .as_ref()
                    .ok_or_else(|| {
                        anyhow::anyhow!("EVM object of the contract `{}:{}` not found", path, name)
                    })?
                    .method_identifiers
                    .as_ref()
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                            "Method identifiers of the contract `{}:{}` not found",
                            path,
                            name
                        )
                    })?
                    .iter()
                {
                    let selector =
                        u32::from_str_radix(selector, era_compiler_common::BASE_HEXADECIMAL)
                            .map_err(|error| {
                                anyhow::anyhow!(
                                    "Invalid selector `{}` received from the Solidity compiler: {}",
                                    selector,
                                    error
                                )
                            })?;
                    contract_identifiers.insert(entry.clone(), selector);
                }
                method_identifiers.insert(format!("{path}:{name}"), contract_identifiers);
            }
        }
        Ok(method_identifiers)
    }

    ///
    /// Get the last contract from the solc output.
    ///
    fn get_last_contract(
        solc_output: &era_compiler_solidity::SolcStandardJsonOutput,
        sources: &[(String, String)],
    ) -> anyhow::Result<String> {
        solc_output
            .sources
            .as_ref()
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "The Solidity sources are empty. Found errors: {:?}",
                    solc_output.errors
                )
            })
            .and_then(|output_sources| {
                for (path, _source) in sources.iter().rev() {
                    match output_sources
                        .get(path)
                        .ok_or_else(|| anyhow::anyhow!("The last source not found in the output"))?
                        .last_contract_name()
                    {
                        Ok(name) => return Ok(format!("{path}:{name}")),
                        Err(_error) => continue,
                    }
                }
                anyhow::bail!("The last source not found in the output")
            })
    }
}

impl Compiler for SolidityCompiler {
    fn compile_for_eravm(
        &self,
        test_path: String,
        sources: Vec<(String, String)>,
        libraries: BTreeMap<String, BTreeMap<String, String>>,
        mode: &Mode,
        llvm_options: Vec<String>,
        debug_config: Option<era_compiler_llvm_context::DebugConfig>,
    ) -> anyhow::Result<EraVMInput> {
        let mode = SolidityMode::unwrap(mode);

        let mut solc_output = self
            .standard_json_output_cached(test_path, &sources, &libraries, mode)
            .map_err(|error| anyhow::anyhow!("Solidity standard JSON I/O error: {}", error))?;
        solc_output.collect_errors()?;

        let method_identifiers = Self::get_method_identifiers(&solc_output)
            .map_err(|error| anyhow::anyhow!("Failed to get method identifiers: {}", error))?;

        let last_contract = Self::get_last_contract(&solc_output, &sources)
            .map_err(|error| anyhow::anyhow!("Failed to get the last contract: {}", error))?;

        let mut solc_compiler = if mode.is_system_contracts_mode {
            SolidityCompiler::system_contract_executable()
        } else {
            SolidityCompiler::executable(&mode.solc_version)
        }?;

        let project = era_compiler_solidity::Project::try_from_solc_output(
            libraries,
            mode.solc_pipeline,
            &mut solc_output,
            &mut solc_compiler,
            debug_config.as_ref(),
        )?;

        let build = project.compile_to_eravm(
            &mut vec![],
            mode.enable_eravm_extensions,
            false,
            mode.llvm_optimizer_settings.to_owned(),
            llvm_options,
            true,
            None,
            debug_config,
        )?;
        build.collect_errors()?;
        let builds = build
            .contracts
            .iter()
            .map(|(path, build)| {
                let build = build.to_owned().expect("Always valid");
                let build = era_compiler_llvm_context::EraVMBuild::new(
                    build.build.bytecode,
                    build.build.bytecode_hash,
                    None,
                    build.build.assembly,
                );
                (path.to_owned(), build)
            })
            .collect();

        build.write_to_standard_json(
            &mut solc_output,
            Some(&era_compiler_solidity::SolcVersion::new(
                mode.solc_version.to_string(),
                mode.solc_version.to_owned(),
                None,
            )),
            &semver::Version::new(0, 0, 0),
        )?;
        solc_output.collect_errors()?;

        Ok(EraVMInput::new(
            builds,
            Some(method_identifiers),
            last_contract,
        ))
    }

    fn compile_for_evm(
        &self,
        test_path: String,
        sources: Vec<(String, String)>,
        libraries: BTreeMap<String, BTreeMap<String, String>>,
        mode: &Mode,
        llvm_options: Vec<String>,
        debug_config: Option<era_compiler_llvm_context::DebugConfig>,
    ) -> anyhow::Result<EVMInput> {
        let mode = SolidityMode::unwrap(mode);

        let mut solc_output =
            self.standard_json_output_cached(test_path, &sources, &libraries, mode)?;
        solc_output.collect_errors()?;

        let method_identifiers = Self::get_method_identifiers(&solc_output)?;

        let last_contract = Self::get_last_contract(&solc_output, &sources)?;

        let mut solc_compiler = SolidityCompiler::executable(&mode.solc_version)?;

        let project = era_compiler_solidity::Project::try_from_solc_output(
            libraries,
            mode.solc_pipeline,
            &mut solc_output,
            &mut solc_compiler,
            debug_config.as_ref(),
        )?;

        let build = project.compile_to_evm(
            &mut vec![],
            mode.llvm_optimizer_settings.to_owned(),
            llvm_options,
            false,
            None,
            debug_config,
        )?;
        build.collect_errors()?;
        let builds: HashMap<String, EVMBuild> = build
            .contracts
            .into_iter()
            .map(|(path, result)| {
                let contract = result.expect("Always valid");
                let build = EVMBuild::new(contract.deploy_build, contract.runtime_build);
                (path, build)
            })
            .collect();

        Ok(EVMInput::new(
            builds,
            Some(method_identifiers),
            last_contract,
        ))
    }

    fn all_modes(&self) -> Vec<Mode> {
        MODES.clone()
    }

    fn allows_multi_contract_files(&self) -> bool {
        true
    }
}
