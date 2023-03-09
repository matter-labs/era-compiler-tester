//!
//! The Solidity compiler wrapper.
//!

pub mod cached_project;
pub mod subprocess_mode;

use std::collections::BTreeMap;
use std::collections::HashMap;
use std::path::Path;

use itertools::Itertools;

use super::build::Build as zkEVMContractBuild;
use super::cache::Cache;
use super::mode::solidity::Mode as SolidityMode;
use super::mode::Mode;
use super::Compiler;

use self::cached_project::CachedProject;
use self::subprocess_mode::SubprocessMode;

///
/// The Solidity compiler wrapper.
///
pub struct SolidityCompiler {
    /// The name-to-code source files mapping.
    sources: Vec<(String, String)>,
    /// The `solc` process output cache.
    cache: Cache<SubprocessMode, anyhow::Result<CachedProject>>,
    /// The libraries addresses.
    libraries: BTreeMap<String, BTreeMap<String, String>>,
    /// The debug config.
    debug_config: Option<compiler_llvm_context::DebugConfig>,
    /// The system mode flag.
    is_system_mode: bool,
}

lazy_static::lazy_static! {
    ///
    /// The Solidity compiler supported modes.
    ///
    /// All compilers must be downloaded before initialization.
    ///
    static ref MODES: Vec<Mode> = {
        let mut solc_pipeline_versions = Vec::new();
        for pipeline in [
            compiler_solidity::SolcPipeline::Yul,
            compiler_solidity::SolcPipeline::EVMLA,
        ] {
            for version in SolidityCompiler::all_versions(pipeline).expect("`solc` versions analysis error") {
                solc_pipeline_versions.push((pipeline, version))
            }
        }

        compiler_llvm_context::OptimizerSettings::combinations()
            .into_iter()
            .cartesian_product(solc_pipeline_versions)
            .cartesian_product(vec![false, true])
            .map(
                |((llvm_optimizer_settings, (solc_pipeline, solc_version)), solc_optimize)| {
                    SolidityMode::new(
                        solc_version,
                        solc_pipeline,
                        solc_optimize,
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
    /// Returns the `solc` compiler path by version.
    ///
    fn get_solc_by_version(version: &semver::Version) -> compiler_solidity::SolcCompiler {
        compiler_solidity::SolcCompiler::new(format!("{}/solc-{}", Self::DIRECTORY, version))
    }

    ///
    /// Returns the system contract `solc` compiler path.
    ///
    fn get_system_contract_solc() -> compiler_solidity::SolcCompiler {
        compiler_solidity::SolcCompiler::new(format!("{}/solc-system-contracts", Self::DIRECTORY))
    }

    ///
    /// Returns the compiler versions downloaded for the specified compilation pipeline.
    ///
    fn all_versions(
        pipeline: compiler_solidity::SolcPipeline,
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
            if compiler_solidity::SolcPipeline::Yul == pipeline && version.minor < 8 {
                continue;
            }
            versions.push(version);
        }
        Ok(versions)
    }

    ///
    /// Processes a project and stores its representation in the cache.
    ///
    fn compute_cache(&self, subprocess_mode: &SubprocessMode, is_system_contracts_mode: bool) {
        if !self.cache.contains(subprocess_mode) {
            if let Some(waiter) = self.cache.start(subprocess_mode.clone()) {
                self.cache.finish(
                    subprocess_mode.clone(),
                    self.get_cached_project(subprocess_mode, is_system_contracts_mode),
                    waiter,
                );
            };
        }
    }

    ///
    /// Processes a project and returns its representation for the cache.
    ///
    fn get_cached_project(
        &self,
        subprocess_mode: &SubprocessMode,
        is_system_contracts_mode: bool,
    ) -> anyhow::Result<CachedProject> {
        let solc = if is_system_contracts_mode {
            Self::get_system_contract_solc()
        } else {
            Self::get_solc_by_version(&subprocess_mode.version)
        };

        let output_selection =
            compiler_solidity::SolcStandardJsonInputSettingsSelection::new_required(
                subprocess_mode.pipeline,
            );

        let optimizer = compiler_solidity::SolcStandardJsonInputSettingsOptimizer::new(
            subprocess_mode.optimize,
            None,
        );

        let solc_input = compiler_solidity::SolcStandardJsonInput::try_from_sources(
            self.sources.clone().into_iter().collect(),
            self.libraries.clone(),
            output_selection,
            optimizer,
        )
        .map_err(|error| anyhow::anyhow!("Failed to build solc input standard json: {}", error))?;

        let allow_paths = Path::new(Self::SOLC_ALLOW_PATHS)
            .canonicalize()
            .expect("Always valid")
            .to_string_lossy()
            .to_string();

        let mut solc_output = solc.standard_json(solc_input, None, vec![], Some(allow_paths))?;

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

        let last_contract = solc_output
            .sources
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Sources not found in the output"))
            .and_then(|sources| {
                for (path, _source) in self.sources.iter().rev() {
                    match sources
                        .get(path)
                        .ok_or_else(|| anyhow::anyhow!("Last source not found in the output"))?
                        .ast
                        .as_ref()
                        .ok_or_else(|| anyhow::anyhow!("AST not found for the last source"))?
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
            })?;

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
                    contract_identifiers.insert(entry.clone(), selector.clone());
                }
                method_identifiers.insert(format!("{path}:{name}"), contract_identifiers);
            }
        }

        let project = solc_output.try_to_project(
            self.sources
                .clone()
                .into_iter()
                .collect::<BTreeMap<String, String>>(),
            self.libraries.clone(),
            subprocess_mode.pipeline,
            &subprocess_mode.version,
            self.debug_config.as_ref(),
        )?;

        Ok(CachedProject::new(
            project,
            method_identifiers,
            last_contract,
        ))
    }
}

impl Compiler for SolidityCompiler {
    fn new(
        sources: Vec<(String, String)>,
        libraries: BTreeMap<String, BTreeMap<String, String>>,
        debug_config: Option<compiler_llvm_context::DebugConfig>,
        is_system_mode: bool,
    ) -> Self {
        Self {
            sources,
            libraries,
            cache: Cache::new(),
            debug_config,
            is_system_mode,
        }
    }

    fn modes() -> Vec<Mode> {
        MODES.clone()
    }

    fn compile(
        &self,
        mode: &Mode,
        is_system_contracts_mode: bool,
    ) -> anyhow::Result<HashMap<String, zkEVMContractBuild>> {
        let mode = SolidityMode::unwrap(mode);

        let subprocess_mode = SubprocessMode::new(
            mode.solc_version.clone(),
            mode.solc_pipeline,
            mode.solc_optimize,
        );

        self.compute_cache(&subprocess_mode, is_system_contracts_mode);

        let project = {
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

        let builds = project
            .compile_all(
                target_machine,
                mode.llvm_optimizer_settings.clone(),
                self.is_system_mode,
                self.debug_config.clone(),
            )?
            .contracts
            .into_iter()
            .map(|(name, contract)| {
                Ok((
                    name,
                    zkEVMContractBuild::new_with_hash(
                        contract.build.assembly,
                        contract.build.bytecode_hash,
                    )?,
                ))
            })
            .collect::<anyhow::Result<HashMap<String, zkEVMContractBuild>>>()?;

        Ok(builds)
    }

    fn selector(
        &self,
        mode: &Mode,
        contract_path: &str,
        entry: &str,
        is_system_contracts_mode: bool,
    ) -> anyhow::Result<u32> {
        let mode = SolidityMode::unwrap(mode);

        let subprocess_mode = SubprocessMode::new(
            mode.solc_version.clone(),
            mode.solc_pipeline,
            mode.solc_optimize,
        );

        self.compute_cache(&subprocess_mode, is_system_contracts_mode);

        self.cache.wait(&subprocess_mode);
        let lock = self.cache.read();
        let cached_project = lock
            .get(&subprocess_mode)
            .expect("Always valid")
            .unwrap_value();

        let method_identifiers = &cached_project
            .as_ref()
            .map_err(|error| anyhow::anyhow!(error.to_string()))?
            .method_identifiers;

        let contract_identifiers = method_identifiers
            .get(contract_path)
            .ok_or_else(|| anyhow::anyhow!("Contract not found"))?;

        contract_identifiers
            .iter()
            .find_map(|(name, selector)| {
                if name.starts_with(entry) {
                    Some(
                        u32::from_str_radix(selector, compiler_common::BASE_HEXADECIMAL).map_err(
                            |error| {
                                anyhow::anyhow!(
                                    "Invalid selector from the Solidity compiler: {}",
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

    fn last_contract(&self, mode: &Mode, is_system_contracts_mode: bool) -> anyhow::Result<String> {
        let mode = SolidityMode::unwrap(mode);

        let subprocess_mode = SubprocessMode::new(
            mode.solc_version.clone(),
            mode.solc_pipeline,
            mode.solc_optimize,
        );

        self.compute_cache(&subprocess_mode, is_system_contracts_mode);

        self.cache.wait(&subprocess_mode);
        let lock = self.cache.read();
        let cached_project = lock
            .get(&subprocess_mode)
            .expect("Always valid")
            .unwrap_value();

        cached_project
            .as_ref()
            .map_err(|error| anyhow::anyhow!(error.to_string()))
            .map(|cached_project| cached_project.last_contract.clone())
    }

    fn has_many_contracts() -> bool {
        true
    }

    fn check_pragmas(&self, mode: &Mode) -> bool {
        let mode = SolidityMode::unwrap(mode);

        self.sources.iter().all(|(_, source_code)| {
            match source_code.lines().find_map(|line| {
                let mut split = line.split_whitespace();
                if let (Some("pragma"), Some("solidity")) = (split.next(), split.next()) {
                    let version = split.join(",").replace(';', "");
                    semver::VersionReq::parse(version.as_str()).ok()
                } else {
                    None
                }
            }) {
                Some(pragma_version_req) => pragma_version_req.matches(&mode.solc_version),
                None => true,
            }
        })
    }

    fn check_ethereum_tests_params(mode: &Mode, params: &solidity_adapter::Params) -> bool {
        if !params.evm_version.matches_any(&[
            solidity_adapter::EVM::TangerineWhistle,
            solidity_adapter::EVM::SpuriousDragon,
            solidity_adapter::EVM::Byzantium,
            solidity_adapter::EVM::Constantinople,
            solidity_adapter::EVM::Petersburg,
            solidity_adapter::EVM::Istanbul,
            solidity_adapter::EVM::Berlin,
            solidity_adapter::EVM::London,
            solidity_adapter::EVM::Paris,
        ]) {
            return false;
        }

        let mode = SolidityMode::unwrap(mode);

        match mode.solc_pipeline {
            compiler_solidity::SolcPipeline::Yul => {
                params.compile_via_yul != solidity_adapter::CompileViaYul::False
                    && params.abi_encoder_v1_only != solidity_adapter::ABIEncoderV1Only::True
            }
            compiler_solidity::SolcPipeline::EVMLA => {
                params.compile_via_yul != solidity_adapter::CompileViaYul::True
            }
        }
    }
}
