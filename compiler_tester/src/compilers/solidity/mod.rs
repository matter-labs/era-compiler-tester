//!
//! The Solidity compiler wrapper.
//!

pub mod solc_cache_key;

use std::collections::BTreeMap;
use std::collections::HashMap;
use std::path::Path;

use itertools::Itertools;

use super::cache::Cache;
use super::mode::solidity::Mode as SolidityMode;
use super::mode::Mode;
use super::Compiler;
use crate::vm::eravm::input::build::Build as EraVMBuild;
use crate::vm::eravm::input::Input as EraVMInput;
use crate::vm::evm::input::build::Build as EVMBuild;
use crate::vm::evm::input::Input as EVMInput;

use self::solc_cache_key::SolcCacheKey;

///
/// The Solidity compiler wrapper.
///
pub struct SolidityCompiler {
    /// The `solc` process output cache.
    cache: Cache<SolcCacheKey, era_compiler_solidity::SolcStandardJsonOutput>,
}

lazy_static::lazy_static! {
    ///
    /// The Solidity compiler supported modes.
    ///
    /// All compilers must be downloaded before initialization.
    ///
    static ref MODES: Vec<Mode> = {
        let mut solc_pipeline_versions = Vec::new();
        for (pipeline, optimize, via_ir) in [
            (era_compiler_solidity::SolcPipeline::Yul, false, true),
            (era_compiler_solidity::SolcPipeline::Yul, true, true),
            (era_compiler_solidity::SolcPipeline::EVMLA, false, false),
            (era_compiler_solidity::SolcPipeline::EVMLA, true, false),
            (era_compiler_solidity::SolcPipeline::EVMLA, true, true),
        ] {
            for version in SolidityCompiler::all_versions(pipeline, via_ir).expect("`solc` versions analysis error") {
                solc_pipeline_versions.push((pipeline, optimize, via_ir, version));
            }
        }

        era_compiler_llvm_context::OptimizerSettings::combinations()
            .into_iter()
            .cartesian_product(solc_pipeline_versions)
            .map(
                |(llvm_optimizer_settings, (pipeline, optimize, via_ir, version))| {
                    SolidityMode::new(
                        version,
                        pipeline,
                        via_ir,
                        optimize,
                        llvm_optimizer_settings,
                    )
                    .into()
                },
            )
            .collect::<Vec<Mode>>()
    };
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
    /// Returns the `solc` compiler path by version.
    ///
    pub fn get_solc_by_version(
        version: &semver::Version,
    ) -> anyhow::Result<era_compiler_solidity::SolcCompiler> {
        era_compiler_solidity::SolcCompiler::new(format!("{}/solc-{}", Self::DIRECTORY, version))
    }

    ///
    /// Returns the system contract `solc` compiler path.
    ///
    pub fn get_system_contract_solc() -> anyhow::Result<era_compiler_solidity::SolcCompiler> {
        era_compiler_solidity::SolcCompiler::new(format!(
            "{}/solc-system-contracts",
            Self::DIRECTORY
        ))
    }

    ///
    /// Returns the compiler versions downloaded for the specified compilation pipeline.
    ///
    pub fn all_versions(
        pipeline: era_compiler_solidity::SolcPipeline,
        via_ir: bool,
    ) -> anyhow::Result<Vec<semver::Version>> {
        let mut versions = Vec::new();
        for entry in std::fs::read_dir("./solc-bin/")? {
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
            if era_compiler_solidity::SolcPipeline::Yul == pipeline && version.minor < 8 {
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
    fn run_solc(
        sources: &[(String, String)],
        libraries: &BTreeMap<String, BTreeMap<String, String>>,
        mode: &SolidityMode,
        is_system_contracts_mode: bool,
    ) -> anyhow::Result<era_compiler_solidity::SolcStandardJsonOutput> {
        let mut solc = if is_system_contracts_mode {
            Self::get_system_contract_solc()
        } else {
            Self::get_solc_by_version(&mode.solc_version)
        }?;

        let output_selection =
            era_compiler_solidity::SolcStandardJsonInputSettingsSelection::new_required(
                mode.solc_pipeline,
            );

        let optimizer = era_compiler_solidity::SolcStandardJsonInputSettingsOptimizer::new(
            mode.solc_optimize,
            None,
            &mode.solc_version,
            false,
            false,
        );

        let evm_version = if mode.solc_version == semver::Version::new(0, 8, 24) {
            Some(era_compiler_common::EVMVersion::Cancun)
        } else {
            None
        };

        let solc_input = era_compiler_solidity::SolcStandardJsonInput::try_from_sources(
            evm_version,
            sources.iter().cloned().collect(),
            libraries.clone(),
            None,
            output_selection,
            optimizer,
            None,
            mode.via_ir,
            None,
        )
        .map_err(|error| anyhow::anyhow!("Failed to build solc input standard json: {}", error))?;

        let allow_paths = Path::new(Self::SOLC_ALLOW_PATHS)
            .canonicalize()
            .expect("Always valid")
            .to_string_lossy()
            .to_string();

        solc.standard_json(
            solc_input,
            mode.solc_pipeline,
            None,
            vec![],
            Some(allow_paths),
        )
    }

    ///
    /// Computes or loads from the cache solc output. Updates the cache if needed.
    ///
    fn run_solc_cached(
        &self,
        test_path: String,
        sources: &[(String, String)],
        libraries: &BTreeMap<String, BTreeMap<String, String>>,
        mode: &SolidityMode,
        is_system_contracts_mode: bool,
    ) -> anyhow::Result<era_compiler_solidity::SolcStandardJsonOutput> {
        let cache_key = SolcCacheKey::new(
            test_path,
            mode.solc_version.clone(),
            mode.solc_pipeline,
            mode.via_ir,
            mode.solc_optimize,
        );

        if !self.cache.contains(&cache_key) {
            self.cache.compute(cache_key.clone(), || {
                Self::run_solc(sources, libraries, mode, is_system_contracts_mode)
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
            .ok_or_else(|| anyhow::anyhow!("Contracts not found in the output"))?;

        let mut method_identifiers = BTreeMap::new();
        for (path, contracts) in files.iter() {
            for (name, contract) in contracts.iter() {
                let mut contract_identifiers = BTreeMap::new();
                for (entry, selector) in contract
                    .evm
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("EVM for contract {}:{} not found", path, name))?
                    .method_identifiers
                    .as_ref()
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                            "Method identifiers for contract {}:{} not found",
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
                                    "Invalid selector from the Solidity compiler: {}",
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
            .ok_or_else(|| anyhow::anyhow!("Sources not found in the output"))
            .and_then(|output_sources| {
                for (path, _source) in sources.iter().rev() {
                    match output_sources
                        .get(path)
                        .ok_or_else(|| anyhow::anyhow!("Last source not found in the output"))?
                        .last_contract_name()
                    {
                        Ok(name) => return Ok(format!("{path}:{name}")),
                        Err(_error) => continue,
                    }
                }
                anyhow::bail!("Last contract not found in all contracts")
            })
            .map_err(|error| {
                anyhow::anyhow!(
                    "Failed to get the last contract: {}, output errors: {:?}",
                    error,
                    solc_output.errors
                )
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
        is_system_mode: bool,
        is_system_contracts_mode: bool,
        debug_config: Option<era_compiler_llvm_context::DebugConfig>,
    ) -> anyhow::Result<EraVMInput> {
        let mode = SolidityMode::unwrap(mode);

        let mut solc_output = self
            .run_solc_cached(
                test_path,
                &sources,
                &libraries,
                mode,
                is_system_contracts_mode,
            )
            .map_err(|error| anyhow::anyhow!("Failed to run solc: {}", error))?;

        if let Some(errors) = solc_output.errors.as_deref() {
            let mut has_errors = false;
            let mut error_messages = Vec::with_capacity(errors.len());

            for error in errors.iter() {
                if error.severity.as_str() == "error" {
                    has_errors = true;
                    error_messages.push(error.formatted_message.to_owned());
                }
            }

            if has_errors {
                anyhow::bail!("Errors found: {:?}", error_messages);
            }
        }

        let method_identifiers = Self::get_method_identifiers(&solc_output)
            .map_err(|error| anyhow::anyhow!("Failed to get method identifiers: {}", error))?;

        let last_contract = Self::get_last_contract(&solc_output, &sources)
            .map_err(|error| anyhow::anyhow!("Failed to get last contract: {}", error))?;

        let project = solc_output.try_to_project(
            sources.into_iter().collect::<BTreeMap<String, String>>(),
            libraries,
            mode.solc_pipeline,
            &era_compiler_solidity::SolcVersion::new_simple(mode.solc_version.to_owned()),
            debug_config.as_ref(),
        )?;

        let build = project.compile_to_eravm(
            mode.llvm_optimizer_settings.to_owned(),
            is_system_mode,
            false,
            zkevm_assembly::get_encoding_mode(),
            debug_config,
        )?;
        build.write_to_standard_json(
            &mut solc_output,
            &era_compiler_solidity::SolcVersion::new(
                mode.solc_version.to_string(),
                mode.solc_version.to_owned(),
                None,
            ),
            &semver::Version::new(0, 0, 0),
        )?;

        let builds: HashMap<String, EraVMBuild> = solc_output
            .contracts
            .expect("Always exists")
            .into_iter()
            .flat_map(|(file_name, file)| {
                file.into_iter()
                    .filter_map(|(contract_name, contract)| {
                        let name = format!("{}:{}", file_name, contract_name);
                        let evm = contract.evm.expect("Always exists");
                        let assembly =
                            zkevm_assembly::Assembly::from_string(evm.assembly_text?, None)
                                .expect("Always valid");
                        let build = match contract.hash {
                            Some(bytecode_hash) => {
                                EraVMBuild::new_with_hash(assembly, bytecode_hash)
                                    .expect("Always valid")
                            }
                            None => EraVMBuild::new(assembly).expect("Always valid"),
                        };
                        Some((name, build))
                    })
                    .collect::<HashMap<String, EraVMBuild>>()
            })
            .collect();

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
        debug_config: Option<era_compiler_llvm_context::DebugConfig>,
    ) -> anyhow::Result<EVMInput> {
        let mode = SolidityMode::unwrap(mode);

        let mut solc_output = self
            .run_solc_cached(test_path, &sources, &libraries, mode, false)
            .map_err(|error| anyhow::anyhow!("Failed to run solc: {}", error))?;

        if let Some(errors) = solc_output.errors.as_deref() {
            let mut has_errors = false;
            let mut error_messages = Vec::with_capacity(errors.len());

            for error in errors.iter() {
                if error.severity.as_str() == "error" {
                    has_errors = true;
                    error_messages.push(error.formatted_message.to_owned());
                }
            }

            if has_errors {
                anyhow::bail!("Errors found: {:?}", error_messages);
            }
        }

        let method_identifiers = Self::get_method_identifiers(&solc_output)
            .map_err(|error| anyhow::anyhow!("Failed to get method identifiers: {}", error))?;

        let last_contract = Self::get_last_contract(&solc_output, &sources)
            .map_err(|error| anyhow::anyhow!("Failed to get last contract: {}", error))?;

        let project = solc_output.try_to_project(
            sources.into_iter().collect::<BTreeMap<String, String>>(),
            libraries,
            mode.solc_pipeline,
            &era_compiler_solidity::SolcVersion::new_simple(mode.solc_version.to_owned()),
            debug_config.as_ref(),
        )?;

        let build =
            project.compile_to_evm(mode.llvm_optimizer_settings.to_owned(), false, debug_config)?;

        let builds: HashMap<String, EVMBuild> = build
            .contracts
            .into_iter()
            .map(|(path, contract)| {
                let build = EVMBuild::new(contract.deploy_build, contract.runtime_build);
                (path, build)
            })
            .collect::<HashMap<String, EVMBuild>>();

        Ok(EVMInput::new(
            builds,
            Some(method_identifiers),
            last_contract,
        ))
    }

    fn modes(&self) -> Vec<Mode> {
        MODES.clone()
    }

    fn has_multiple_contracts(&self) -> bool {
        true
    }
}
