//!
//! The upstream Solidity compiler.
//!

pub mod mode;
pub mod solc;

use std::collections::BTreeMap;
use std::collections::HashMap;
use std::path::Path;

use crate::compilers::cache::Cache;
use crate::compilers::mode::Mode;
use crate::compilers::solidity::cache_key::CacheKey;
use crate::compilers::Compiler;
use crate::vm::eravm::input::Input as EraVMInput;
use crate::vm::evm::input::build::Build as EVMBuild;
use crate::vm::evm::input::Input as EVMInput;

use self::mode::Mode as SolidityUpstreamMode;
use self::solc::standard_json::input::settings::optimizer::Optimizer as SolcStandardJsonInputSettingsOptimizer;
use self::solc::standard_json::input::settings::selection::Selection as SolcStandardJsonInputSettingsSelection;
use self::solc::standard_json::input::Input as SolcStandardJsonInput;
use self::solc::standard_json::output::Output as SolcStandardJsonOutput;
use self::solc::Compiler as SolcUpstreamCompiler;

///
/// The upstream Solidity compiler.
///
pub struct SolidityCompiler {
    /// The `solc` process output cache.
    cache: Cache<CacheKey, SolcStandardJsonOutput>,
}

lazy_static::lazy_static! {
    ///
    /// The Solidity compiler supported modes.
    ///
    /// All compilers must be downloaded before initialization.
    ///
    static ref MODES: Vec<Mode> = {
        let mut modes = Vec::new();
        for (pipeline, optimize, via_ir) in [
            (era_compiler_solidity::SolcPipeline::EVMLA, false, false),
            (era_compiler_solidity::SolcPipeline::EVMLA, false, true),
            (era_compiler_solidity::SolcPipeline::EVMLA, true, false),
            (era_compiler_solidity::SolcPipeline::EVMLA, true, true),
            (era_compiler_solidity::SolcPipeline::Yul, false, true),
            (era_compiler_solidity::SolcPipeline::Yul, true, true),
        ] {
            for version in SolidityCompiler::all_versions(pipeline, via_ir).expect("`solc` versions analysis error") {
                modes.push(SolidityUpstreamMode::new(version, pipeline, via_ir, optimize).into());
            }
        }
        modes
    };
}

impl Default for SolidityCompiler {
    fn default() -> Self {
        Self::new()
    }
}

impl SolidityCompiler {
    /// The compiler binaries directory.
    const DIRECTORY: &'static str = "solc-bin-upstream/";

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
    pub fn executable(version: &semver::Version) -> anyhow::Result<SolcUpstreamCompiler> {
        SolcUpstreamCompiler::new(format!("{}/solc-{}", Self::DIRECTORY, version))
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
                && version < SolcUpstreamCompiler::FIRST_YUL_VERSION
            {
                continue;
            }
            if era_compiler_solidity::SolcPipeline::EVMLA == pipeline
                && via_ir
                && version < SolcUpstreamCompiler::FIRST_VIA_IR_VERSION
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
        mode: &SolidityUpstreamMode,
    ) -> anyhow::Result<SolcStandardJsonOutput> {
        let mut solc = Self::executable(&mode.solc_version)?;

        let output_selection =
            SolcStandardJsonInputSettingsSelection::new_required(mode.solc_pipeline);

        let optimizer = SolcStandardJsonInputSettingsOptimizer::new(mode.solc_optimize);

        let evm_version = if mode.solc_version >= SolcUpstreamCompiler::FIRST_CANCUN_VERSION {
            Some(era_compiler_common::EVMVersion::Cancun)
        } else {
            None
        };

        let solc_input = SolcStandardJsonInput::try_from_sources(
            evm_version,
            sources.iter().cloned().collect(),
            libraries.clone(),
            None,
            output_selection,
            optimizer,
            mode.via_ir,
        )
        .map_err(|error| anyhow::anyhow!("Solidity standard JSON I/O error: {}", error))?;

        let allow_paths = Path::new(Self::SOLC_ALLOW_PATHS)
            .canonicalize()
            .expect("Always valid")
            .to_string_lossy()
            .to_string();

        solc.standard_json(solc_input, None, vec![], Some(allow_paths))
    }

    ///
    /// Evaluates the standard JSON output or loads it from the cache.
    ///
    fn standard_json_output_cached(
        &self,
        test_path: String,
        sources: &[(String, String)],
        libraries: &BTreeMap<String, BTreeMap<String, String>>,
        mode: &SolidityUpstreamMode,
    ) -> anyhow::Result<SolcStandardJsonOutput> {
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
        solc_output: &SolcStandardJsonOutput,
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
        solc_output: &SolcStandardJsonOutput,
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
        _test_path: String,
        _sources: Vec<(String, String)>,
        _libraries: BTreeMap<String, BTreeMap<String, String>>,
        _mode: &Mode,
        _llvm_options: Vec<String>,
        _debug_config: Option<era_compiler_llvm_context::DebugConfig>,
    ) -> anyhow::Result<EraVMInput> {
        anyhow::bail!("The upstream Solidity compiler cannot compile for EraVM");
    }

    fn compile_for_evm(
        &self,
        test_path: String,
        sources: Vec<(String, String)>,
        libraries: BTreeMap<String, BTreeMap<String, String>>,
        mode: &Mode,
        _llvm_options: Vec<String>,
        _debug_config: Option<era_compiler_llvm_context::DebugConfig>,
    ) -> anyhow::Result<EVMInput> {
        let mode = SolidityUpstreamMode::unwrap(mode);

        let solc_output =
            self.standard_json_output_cached(test_path, &sources, &libraries, mode)?;

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
                anyhow::bail!("`solc` errors found: {:?}", error_messages);
            }
        }

        let method_identifiers = Self::get_method_identifiers(&solc_output)?;

        let last_contract = Self::get_last_contract(&solc_output, &sources)?;

        let contracts = solc_output
            .contracts
            .ok_or_else(|| anyhow::anyhow!("Solidity contracts not found in the output"))?;

        let mut builds = HashMap::with_capacity(contracts.len());
        for (file, contracts) in contracts.into_iter() {
            for (name, contract) in contracts.into_iter() {
                let path = format!("{file}:{name}");
                let bytecode_string = contract
                    .evm
                    .as_ref()
                    .ok_or_else(|| {
                        anyhow::anyhow!("EVM object of the contract `{path}` not found")
                    })?
                    .bytecode
                    .as_ref()
                    .ok_or_else(|| {
                        anyhow::anyhow!("EVM bytecode of the contract `{path}` not found")
                    })?
                    .object
                    .as_str();
                let build = EVMBuild::new(
                    era_compiler_llvm_context::EVMBuild::new(
                        hex::decode(bytecode_string).expect("Always valid"),
                        None,
                    ),
                    era_compiler_llvm_context::EVMBuild::default(),
                );
                builds.insert(path, build);
            }
        }

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
