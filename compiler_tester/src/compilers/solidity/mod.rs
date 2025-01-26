//!
//! The Solidity compiler.
//!

pub mod cache_key;
pub mod mode;
pub mod upstream;

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::collections::HashMap;
use std::path::Path;

use itertools::Itertools;

use era_solc::CollectableError;

use crate::compilers::cache::Cache;
use crate::compilers::mode::Mode;
use crate::compilers::Compiler;
use crate::vm::eravm::input::Input as EraVMInput;
use crate::vm::revm::input::build::Build as EVMBuild;
use crate::vm::revm::input::Input as EVMInput;

use self::cache_key::CacheKey;
use self::mode::Mode as SolidityMode;

///
/// The Solidity compiler.
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
        for (codegen, optimize, via_ir) in [
            (era_solc::StandardJsonInputCodegen::EVMLA, true, false),
            (era_solc::StandardJsonInputCodegen::EVMLA, true, true),
            (era_solc::StandardJsonInputCodegen::Yul, true, true),
        ] {
            for version in SolidityCompiler::all_versions(codegen, via_ir).expect("`solc` versions analysis error") {
                solc_codegen_versions.push((codegen, optimize, via_ir, version));
            }
        }

        era_compiler_llvm_context::OptimizerSettings::combinations()
            .into_iter()
            .cartesian_product(solc_codegen_versions)
            .map(
                |(mut llvm_optimizer_settings, (codegen, optimize, via_ir, version))| {
                    llvm_optimizer_settings.enable_fallback_to_size();
                    SolidityMode::new(
                        version,
                        codegen,
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
    /// The last ZKsync `solc` revision.
    pub const LAST_ZKSYNC_SOLC_REVISION: semver::Version = semver::Version::new(1, 0, 1);

    /// The compiler executables directory.
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
    pub fn executable(version: &semver::Version) -> anyhow::Result<era_solc::Compiler> {
        era_solc::Compiler::try_from_path(format!("{}/solc-{}", Self::DIRECTORY, version).as_str())
    }

    ///
    /// Returns the `solc` executable used to compile system contracts.
    ///
    pub fn system_contract_executable() -> anyhow::Result<era_solc::Compiler> {
        era_solc::Compiler::try_from_path(
            format!("{}/solc-system-contracts", Self::DIRECTORY).as_str(),
        )
    }

    ///
    /// Returns the compiler versions downloaded for the specified compilation codegen.
    ///
    pub fn all_versions(
        codegen: era_solc::StandardJsonInputCodegen,
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
                    "Invalid `solc` executable file type: {}",
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
            if era_solc::StandardJsonInputCodegen::Yul == codegen
                && version < era_solc::Compiler::FIRST_YUL_VERSION
            {
                continue;
            }
            if era_solc::StandardJsonInputCodegen::EVMLA == codegen
                && via_ir
                && version < era_solc::Compiler::FIRST_VIA_IR_VERSION
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
        libraries: &era_solc::StandardJsonInputLibraries,
        mode: &SolidityMode,
    ) -> anyhow::Result<era_solc::StandardJsonOutput> {
        let solc_compiler = if mode.is_system_contracts_mode {
            Self::system_contract_executable()
        } else {
            Self::executable(&mode.solc_version)
        }?;

        let mut output_selection =
            era_solc::StandardJsonInputSelection::new_required(mode.solc_codegen);
        output_selection.extend(era_solc::StandardJsonInputSelection::new(vec![
            era_solc::StandardJsonInputSelector::EraVMAssembly,
        ]));

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

        let mut solc_input = era_solc::StandardJsonInput::try_from_solidity_sources(
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
            vec![era_solc::StandardJsonInputErrorType::SendTransfer],
            vec![era_solc::StandardJsonInputWarningType::AssemblyCreate],
            false,
            mode.via_ir,
        )
        .map_err(|error| anyhow::anyhow!("Solidity standard JSON I/O error: {}", error))?;

        let allow_paths = Path::new(Self::SOLC_ALLOW_PATHS)
            .canonicalize()
            .expect("Always valid")
            .to_string_lossy()
            .to_string();

        solc_compiler.standard_json(
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
        mode: &SolidityMode,
    ) -> anyhow::Result<era_solc::StandardJsonOutput> {
        let cache_key = CacheKey::new(
            test_path,
            mode.solc_version.clone(),
            mode.solc_codegen,
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
        test_path: String,
        sources: Vec<(String, String)>,
        libraries: era_solc::StandardJsonInputLibraries,
        mode: &Mode,
        llvm_options: Vec<String>,
        debug_config: Option<era_compiler_llvm_context::DebugConfig>,
    ) -> anyhow::Result<EraVMInput> {
        let mode = SolidityMode::unwrap(mode);

        let mut solc_output = self
            .standard_json_output_cached(test_path, &sources, &libraries, mode)
            .map_err(|error| anyhow::anyhow!("Solidity standard JSON I/O error: {}", error))?;
        solc_output.check_errors()?;

        let method_identifiers = Self::get_method_identifiers(&solc_output)
            .map_err(|error| anyhow::anyhow!("Failed to get method identifiers: {}", error))?;

        let last_contract = Self::get_last_contract(&solc_output, &sources)
            .map_err(|error| anyhow::anyhow!("Failed to get the last contract: {}", error))?;

        let solc_compiler = if mode.is_system_contracts_mode {
            SolidityCompiler::system_contract_executable()
        } else {
            SolidityCompiler::executable(&mode.solc_version)
        }?;

        let linker_symbols = libraries.as_linker_symbols()?;

        let project = era_compiler_solidity::Project::try_from_solc_output(
            libraries,
            mode.solc_codegen,
            &mut solc_output,
            &solc_compiler,
            debug_config.as_ref(),
        )?;

        let build = project.compile_to_eravm(
            &mut vec![],
            mode.enable_eravm_extensions,
            era_compiler_common::HashType::Ipfs,
            mode.llvm_optimizer_settings.to_owned(),
            llvm_options,
            true,
            debug_config,
        )?;
        build.check_errors()?;
        let build = build.link(linker_symbols);
        build.check_errors()?;
        let builds = build
            .results
            .iter()
            .map(|(path, build)| {
                let build = build.to_owned().expect("Always valid");
                let build = era_compiler_llvm_context::EraVMBuild::new_with_bytecode_hash(
                    build.build.bytecode,
                    build.build.bytecode_hash.ok_or_else(|| {
                        anyhow::anyhow!("Bytecode hash not found in the build artifacts")
                    })?,
                    None,
                    build.build.assembly,
                );
                Ok((path.to_owned(), build))
            })
            .collect::<anyhow::Result<HashMap<String, era_compiler_llvm_context::EraVMBuild>>>()?;

        build.write_to_standard_json(
            &mut solc_output,
            Some(&era_solc::Version::new(
                mode.solc_version.to_string(),
                mode.solc_version.to_owned(),
                Self::LAST_ZKSYNC_SOLC_REVISION,
            )),
        )?;
        solc_output.check_errors()?;

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
        libraries: era_solc::StandardJsonInputLibraries,
        mode: &Mode,
        _test_params: Option<&solidity_adapter::Params>,
        llvm_options: Vec<String>,
        debug_config: Option<era_compiler_llvm_context::DebugConfig>,
    ) -> anyhow::Result<EVMInput> {
        let mode = SolidityMode::unwrap(mode);

        let mut solc_output =
            self.standard_json_output_cached(test_path, &sources, &libraries, mode)?;
        solc_output.check_errors()?;

        let method_identifiers = Self::get_method_identifiers(&solc_output)?;

        let last_contract = Self::get_last_contract(&solc_output, &sources)?;

        let solc_compiler = SolidityCompiler::executable(&mode.solc_version)?;

        let linker_symbols = libraries.as_linker_symbols()?;

        let project = era_compiler_solidity::Project::try_from_solc_output(
            libraries,
            mode.solc_codegen,
            &mut solc_output,
            &solc_compiler,
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
        let builds: HashMap<String, EVMBuild> = build
            .results
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
