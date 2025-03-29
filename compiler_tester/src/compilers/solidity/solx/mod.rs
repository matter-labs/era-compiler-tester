//!
//! The `solx` compiler.
//!

pub mod mode;

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::collections::HashMap;
use std::path::Path;

use itertools::Itertools;

use era_solc::CollectableError;

use crate::compilers::cache::Cache;
use crate::compilers::mode::Mode;
use crate::compilers::solidity::cache_key::CacheKey;
use crate::compilers::Compiler;
use crate::vm::eravm::input::Input as EraVMInput;
use crate::vm::revm::input::Input as EVMInput;

use self::mode::Mode as SolxMode;

///
/// The `solx` compiler.
///
pub struct SolidityCompiler {
    /// The `solc` process output cache.
    cache: Cache<CacheKey, era_solc::StandardJsonOutput>,
}

lazy_static::lazy_static! {
    ///
    /// All supported modes.
    ///
    /// All compilers must be downloaded before initialization.
    ///
    static ref MODES: Vec<Mode> = {
        let mut solc_codegen_versions = Vec::new();
        for (codegen, via_ir) in [
            (era_solc::StandardJsonInputCodegen::EVMLA, false),
            (era_solc::StandardJsonInputCodegen::EVMLA, true),
            (era_solc::StandardJsonInputCodegen::Yul, true),
        ] {
            solc_codegen_versions.push((codegen, via_ir, solx_solc::SolcCompiler::default().version));
        }

        era_compiler_llvm_context::OptimizerSettings::combinations()
            .into_iter()
            .cartesian_product(solc_codegen_versions)
            .map(
                |(mut llvm_optimizer_settings, (codegen, via_ir, version))| {
                    llvm_optimizer_settings.enable_fallback_to_size();
                    SolxMode::new(
                        version,
                        via_ir,
                        llvm_optimizer_settings,
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
    /// Runs the solc subprocess and returns the output.
    ///
    fn standard_json_output(
        sources: &[(String, String)],
        libraries: &era_solc::StandardJsonInputLibraries,
        mode: &SolxMode,
    ) -> anyhow::Result<era_solc::StandardJsonOutput> {
        let mut output_selection = solx_solc::StandardJsonInputSelection::new(mode.solc_codegen);

        let evm_version = if mode.solc_version >= era_solc::Compiler::FIRST_CANCUN_VERSION {
            Some(era_compiler_common::EVMVersion::Cancun)
        } else {
            None
        };

        let sources: BTreeMap<String, era_solc::StandardJsonInputSource> = sources
            .iter()
            .map(|(path, source)| {
                (
                    path.to_owned(),
                    era_solc::StandardJsonInputSource::from(source.to_owned()),
                )
            })
            .collect();

        let mut solc_input = solx_solc::StandardJsonInput::try_from_solidity_sources(
            sources,
            libraries.to_owned(),
            BTreeSet::new(),
            era_solc::StandardJsonInputOptimizer::default(),
            Some(mode.solc_codegen),
            evm_version,
            mode.enable_eravm_extensions,
            output_selection,
            era_solc::StandardJsonInputMetadata::default(),
            vec![],
            mode.via_ir,
        )
        .map_err(|error| anyhow::anyhow!("Solidity standard JSON I/O error: {}", error))?;

        let allow_paths = Path::new(Self::SOLC_ALLOW_PATHS)
            .canonicalize()
            .expect("Always valid")
            .to_string_lossy()
            .to_string();

        solx_solc::SolcCompiler::default().standard_json(
            target,
            &mut solc_input,
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
        libraries: &era_solc::StandardJsonInputLibraries,
        mode: &SolxMode,
    ) -> anyhow::Result<era_solc::StandardJsonOutput> {
        let cache_key = CacheKey::new(
            test_path,
            mode.solc_version.clone(),
            None,
            mode.via_ir,
            true,
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
        solc_output: &era_solc::StandardJsonOutput,
    ) -> anyhow::Result<BTreeMap<String, BTreeMap<String, u32>>> {
        let mut method_identifiers = BTreeMap::new();
        for (path, file) in solc_output.contracts.iter() {
            for (name, contract) in file.iter() {
                let mut contract_identifiers = BTreeMap::new();
                for (entry, selector) in contract
                    .evm
                    .as_ref()
                    .ok_or_else(|| {
                        anyhow::anyhow!("EVM object of the contract `{}:{}` not found", path, name)
                    })?
                    .method_identifiers
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
        solc_output: &era_solc::StandardJsonOutput,
        sources: &[(String, String)],
    ) -> anyhow::Result<String> {
        for (path, _source) in sources.iter().rev() {
            match solc_output
                .sources
                .get(path)
                .ok_or_else(|| anyhow::anyhow!("The last source not found in the output"))?
                .last_contract_name()
            {
                Ok(name) => return Ok(format!("{path}:{name}")),
                Err(_error) => continue,
            }
        }
        anyhow::bail!("The last source not found in the output")
    }
}

impl Compiler for SolidityCompiler {
    fn compile_for_eravm(
        &self,
        _test_path: String,
        _sources: Vec<(String, String)>,
        _libraries: era_solc::StandardJsonInputLibraries,
        _mode: &Mode,
        _llvm_options: Vec<String>,
        _debug_config: Option<era_compiler_llvm_context::DebugConfig>,
    ) -> anyhow::Result<EraVMInput> {
        panic!("`solx` compiler does not support compilation to EraVM");
    }

    fn compile_for_evm(
        &self,
        test_path: String,
        sources: Vec<(String, String)>,
        libraries: era_solc::StandardJsonInputLibraries,
        mode: &Mode,
        _test_params: Option<&solidity_adapter::Params>,
        llvm_options: Vec<String>,
        debug_config: Option<era_compiler_llvm_context::DebugConfig>,
    ) -> anyhow::Result<EVMInput> {
        let mode = SolxMode::unwrap(mode);

        let mut solc_output =
            self.standard_json_output_cached(test_path, &sources, &libraries, mode)?;
        solc_output.check_errors()?;

        let method_identifiers = Self::get_method_identifiers(&solc_output)?;

        let last_contract = Self::get_last_contract(&solc_output, &sources)?;

        let linker_symbols = libraries.as_linker_symbols()?;

        let project = solx::Project::try_from_solc_output(
            libraries,
            mode.via_ir,
            &mut solc_output,
            debug_config.as_ref(),
        )?;

        let build = project.compile_to_evm(
            &mut vec![],
            era_compiler_common::HashType::Ipfs,
            mode.llvm_optimizer_settings.to_owned(),
            llvm_options,
            debug_config,
        )?;
        build.check_errors()?;
        let build = build.link(linker_symbols);
        build.check_errors()?;
        let builds: HashMap<String, Vec<u8>> = build
            .results
            .into_iter()
            .map(|(path, result)| (path, result.expect("Always valid").deploy_object.bytecode))
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
