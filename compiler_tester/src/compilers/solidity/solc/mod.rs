//!
//! The `solc` Solidity compiler.
//!

pub mod compiler;
pub mod mode;

use std::collections::BTreeMap;
use std::collections::HashMap;
use std::path::Path;

use crate::compilers::cache::Cache;
use crate::compilers::mode::Mode;
use crate::compilers::solidity::cache_key::CacheKey;
use crate::compilers::yul::mode_upstream::Mode as YulUpstreamMode;
use crate::compilers::Compiler;
use crate::toolchain::Toolchain;
use crate::vm::eravm::input::Input as EraVMInput;
use crate::vm::revm::input::Input as EVMInput;

use self::compiler::standard_json::input::language::Language as SolcStandardJsonInputLanguage;
use self::compiler::standard_json::input::settings::debug::Debug as SolcStandardJsonInputDebug;
use self::compiler::standard_json::input::settings::optimizer::Optimizer as SolcStandardJsonInputOptimizer;
use self::compiler::standard_json::input::settings::selection::Selection as SolcStandardJsonInputSelection;
use self::compiler::standard_json::input::Input as SolcStandardJsonInput;
use self::compiler::standard_json::output::Output as SolcStandardJsonOutput;
use self::compiler::Compiler as SolcUpstreamCompiler;
use self::mode::Mode as SolcMode;

///
/// The `solc` Solidity compiler.
///
pub struct SolidityCompiler {
    /// The language the compiler will compile.
    language: SolcStandardJsonInputLanguage,
    /// The toolchain identifier.
    /// Only `solc` and `solc-llvm` are supported.
    toolchain: Toolchain,
    /// The `solc` process output cache.
    cache: Cache<CacheKey, SolcStandardJsonOutput>,
}

lazy_static::lazy_static! {
    ///
    /// The Solidity compiler supported modes.
    ///
    /// All compilers must be downloaded before initialization.
    ///
    static ref SOLIDITY_MODES: Vec<Mode> = {
        let mut modes = Vec::new();
        for (codegen, optimize, via_ir) in [
            (era_solc::StandardJsonInputCodegen::EVMLA, false, false),
            (era_solc::StandardJsonInputCodegen::EVMLA, false, true),
            (era_solc::StandardJsonInputCodegen::EVMLA, true, false),
            (era_solc::StandardJsonInputCodegen::EVMLA, true, true),
            (era_solc::StandardJsonInputCodegen::Yul, false, true),
            (era_solc::StandardJsonInputCodegen::Yul, true, true),
        ] {
            for version in SolidityCompiler::all_versions(codegen, via_ir).expect("`solc` versions analysis error") {
                modes.push(SolcMode::new(version, codegen, via_ir, false, optimize).into());
            }
        }
        modes
    };

    ///
    /// The Yul compiler supported modes.
    ///
    /// All compilers must be downloaded before initialization.
    ///
    static ref YUL_MODES: Vec<Mode> = {
        let mut modes = Vec::new();
        for optimize in [
            false, true
        ] {
            for version in SolidityCompiler::all_versions(era_solc::StandardJsonInputCodegen::Yul, true).expect("`solc` versions analysis error") {
                modes.push(YulUpstreamMode::new(version, false, optimize).into());
            }
        }
        modes
    };

    ///
    /// The supported Solidity modes for MLIR codegen.
    ///
    /// All compilers must be downloaded before initialization.
    ///
    static ref SOLIDITY_MLIR_MODES: Vec<Mode> = {
        vec![SolcMode::new(semver::Version::new(0, 8, 26), era_solc::StandardJsonInputCodegen::Yul, false, true, false).into()]
    };

    ///
    /// The supported Yul modes for MLIR codegen.
    ///
    /// All compilers must be downloaded before initialization.
    ///
    static ref YUL_MLIR_MODES: Vec<Mode> = {
        vec![YulUpstreamMode::new(semver::Version::new(0, 8, 26), true, false).into()]
    };
}

impl SolidityCompiler {
    /// The upstream compiler executables directory.
    const DIRECTORY_UPSTREAM: &'static str = "solc-bin-upstream/";

    /// The LLVM-fork compiler executables directory.
    const DIRECTORY_LLVM: &'static str = "solc-bin-llvm/";

    /// The solc allow paths argument value.
    const SOLC_ALLOW_PATHS: &'static str = "tests";

    ///
    /// A shortcut constructor.
    ///
    pub fn new(language: SolcStandardJsonInputLanguage, toolchain: Toolchain) -> Self {
        Self {
            language,
            toolchain,
            cache: Cache::new(),
        }
    }

    ///
    /// Returns the `solc` executable by its version.
    ///
    pub fn executable(
        toolchain: Toolchain,
        version: &semver::Version,
    ) -> anyhow::Result<SolcUpstreamCompiler> {
        let directory = match toolchain {
            Toolchain::Solc => Self::DIRECTORY_UPSTREAM,
            Toolchain::SolcLLVM => Self::DIRECTORY_LLVM,
            toolchain => panic!("Unsupported toolchain: {toolchain}"),
        };
        SolcUpstreamCompiler::new(format!("{}/solc-{}", directory, version))
    }

    ///
    /// Returns the compiler versions downloaded for the specified compilation codegen.
    ///
    pub fn all_versions(
        codegen: era_solc::StandardJsonInputCodegen,
        via_ir: bool,
    ) -> anyhow::Result<Vec<semver::Version>> {
        let mut versions = Vec::new();
        for entry in std::fs::read_dir(Self::DIRECTORY_UPSTREAM)? {
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
                && version < SolcUpstreamCompiler::FIRST_YUL_VERSION
            {
                continue;
            }
            if era_solc::StandardJsonInputCodegen::EVMLA == codegen
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
    pub fn standard_json_output(
        language: SolcStandardJsonInputLanguage,
        toolchain: Toolchain,
        sources: &[(String, String)],
        libraries: &era_compiler_common::Libraries,
        mode: &Mode,
        test_params: Option<&solidity_adapter::Params>,
    ) -> anyhow::Result<SolcStandardJsonOutput> {
        let solc_version = match mode {
            Mode::Solc(mode) => &mode.solc_version,
            Mode::YulUpstream(mode) => &mode.solc_version,
            mode => anyhow::bail!("Unsupported mode: {mode}"),
        };
        let mut solc = Self::executable(toolchain, solc_version)?;

        let output_selection = SolcStandardJsonInputSelection::new_required(match mode {
            Mode::Solc(mode) => mode.solc_codegen,
            Mode::YulUpstream(_mode) => era_solc::StandardJsonInputCodegen::Yul,
            mode => anyhow::bail!("Unsupported mode: {mode}"),
        });

        let optimizer = SolcStandardJsonInputOptimizer::new(match mode {
            Mode::Solc(mode) => mode.solc_optimize,
            Mode::YulUpstream(mode) => mode.solc_optimize,
            mode => anyhow::bail!("Unsupported mode: {mode}"),
        });

        let evm_version = match mode {
            Mode::Solc(mode) if mode.solc_version >= SolcUpstreamCompiler::FIRST_CANCUN_VERSION => {
                Some(era_compiler_common::EVMVersion::Cancun)
            }
            Mode::Solc(_mode) => None,
            Mode::YulUpstream(_mode) => Some(era_compiler_common::EVMVersion::Cancun),
            mode => anyhow::bail!("Unsupported mode: {mode}"),
        };

        let via_ir = match mode {
            Mode::Solc(mode) => mode.via_ir,
            Mode::YulUpstream(_mode) => true,
            mode => anyhow::bail!("Unsupported mode: {mode}"),
        };

        let via_mlir = match mode {
            Mode::Solc(mode) => mode.via_mlir,
            Mode::YulUpstream(mode) => mode.via_mlir,
            mode => anyhow::bail!("Unsupported mode: {mode}"),
        };

        let debug = if solc_version >= &semver::Version::new(0, 6, 3) {
            test_params.map(|test_params| {
                SolcStandardJsonInputDebug::new(Some(test_params.revert_strings.to_string()))
            })
        } else {
            None
        };

        let solc_input = SolcStandardJsonInput::try_from_sources(
            language,
            evm_version,
            sources.iter().cloned().collect(),
            libraries.clone(),
            None,
            output_selection,
            via_ir,
            via_mlir,
            optimizer,
            debug,
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
    pub fn standard_json_output_cached(
        &self,
        test_path: String,
        language: SolcStandardJsonInputLanguage,
        sources: &[(String, String)],
        libraries: &era_compiler_common::Libraries,
        mode: &Mode,
        test_params: Option<&solidity_adapter::Params>,
    ) -> anyhow::Result<SolcStandardJsonOutput> {
        let cache_key = match mode {
            Mode::Solc(mode) => CacheKey::new(
                test_path,
                mode.solc_version.to_owned(),
                Some(mode.solc_codegen),
                mode.via_ir,
                mode.solc_optimize,
            ),
            Mode::YulUpstream(mode) => CacheKey::new(
                test_path,
                mode.solc_version.to_owned(),
                Some(era_solc::StandardJsonInputCodegen::Yul),
                true,
                mode.solc_optimize,
            ),
            mode => anyhow::bail!("Unsupported mode: {mode}"),
        };

        if !self.cache.contains(&cache_key) {
            self.cache.evaluate(cache_key.clone(), || {
                Self::standard_json_output(
                    language,
                    self.toolchain,
                    sources,
                    libraries,
                    mode,
                    test_params,
                )
            });
        }

        self.cache.get_cloned(&cache_key)
    }

    ///
    /// Get the method identifiers from the solc output.
    ///
    pub fn get_method_identifiers(
        solc_output: &SolcStandardJsonOutput,
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
    pub fn get_last_contract(
        language: SolcStandardJsonInputLanguage,
        solc_output: &SolcStandardJsonOutput,
        sources: &[(String, String)],
    ) -> anyhow::Result<String> {
        match language {
            SolcStandardJsonInputLanguage::Solidity => solc_output
                .sources
                .as_ref()
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "The sources are empty. Found errors: {:?}",
                        solc_output.errors
                    )
                })
                .and_then(|output_sources| {
                    for (path, _source) in sources.iter().rev() {
                        match output_sources
                            .get(path)
                            .ok_or_else(|| {
                                anyhow::anyhow!("The last source not found in the output")
                            })?
                            .last_contract_name()
                        {
                            Ok(name) => return Ok(format!("{path}:{name}")),
                            Err(_error) => continue,
                        }
                    }
                    anyhow::bail!("The last source not found in the output")
                }),
            SolcStandardJsonInputLanguage::Yul => solc_output
                .contracts
                .as_ref()
                .and_then(|contracts| contracts.first_key_value())
                .and_then(|(path, contracts)| {
                    contracts
                        .first_key_value()
                        .map(|(name, _contract)| format!("{path}:{name}"))
                })
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "The sources are empty. Found errors: {:?}",
                        solc_output.errors
                    )
                }),
        }
    }
}

impl Compiler for SolidityCompiler {
    fn compile_for_eravm(
        &self,
        test_path: String,
        sources: Vec<(String, String)>,
        libraries: era_compiler_common::Libraries,
        mode: &Mode,
        _llvm_options: Vec<String>,
        _debug_config: Option<era_compiler_llvm_context::DebugConfig>,
    ) -> anyhow::Result<EraVMInput> {
        let solc_output = self.standard_json_output_cached(
            test_path,
            self.language,
            &sources,
            &libraries,
            mode,
            None,
        )?;

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

        let method_identifiers = match self.language {
            SolcStandardJsonInputLanguage::Solidity => {
                Some(Self::get_method_identifiers(&solc_output)?)
            }
            SolcStandardJsonInputLanguage::Yul => None,
        };

        let last_contract = Self::get_last_contract(self.language, &solc_output, &sources)?;

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
                        anyhow::anyhow!("EraVM object of the contract `{path}` not found")
                    })?
                    .bytecode
                    .as_ref()
                    .ok_or_else(|| {
                        anyhow::anyhow!("EraVM bytecode of the contract `{path}` not found")
                    })?
                    .object
                    .as_str();
                let bytecode = hex::decode(bytecode_string).map_err(|error| {
                    anyhow::anyhow!("EraVM bytecode of the contract `{path}` is invalid: {error}")
                })?;
                let bytecode_words: Vec<[u8; era_compiler_common::BYTE_LENGTH_FIELD]> = bytecode
                    .as_slice()
                    .chunks(era_compiler_common::BYTE_LENGTH_FIELD)
                    .map(|word| word.try_into().expect("Always valid"))
                    .collect();
                let bytecode_hash = zkevm_opcode_defs::utils::bytecode_to_code_hash_for_mode::<
                    { era_compiler_common::BYTE_LENGTH_X64 },
                    zkevm_opcode_defs::decoding::EncodingModeProduction,
                >(bytecode_words.as_slice())
                .map_err(|_| {
                    anyhow::anyhow!("EraVM bytecode of the contract `{path}` hashing error")
                })?;
                let build = era_compiler_llvm_context::EraVMBuild::new_with_bytecode_hash(
                    bytecode,
                    bytecode_hash,
                    None,
                    None,
                );
                builds.insert(path, build);
            }
        }

        Ok(EraVMInput::new(builds, method_identifiers, last_contract))
    }

    fn compile_for_evm(
        &self,
        test_path: String,
        sources: Vec<(String, String)>,
        libraries: era_compiler_common::Libraries,
        mode: &Mode,
        test_params: Option<&solidity_adapter::Params>,
        _llvm_options: Vec<String>,
        _debug_config: Option<era_compiler_llvm_context::DebugConfig>,
    ) -> anyhow::Result<EVMInput> {
        let solc_output = self.standard_json_output_cached(
            test_path,
            self.language,
            &sources,
            &libraries,
            mode,
            test_params,
        )?;

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

        let method_identifiers = match self.language {
            SolcStandardJsonInputLanguage::Solidity => {
                Some(Self::get_method_identifiers(&solc_output)?)
            }
            SolcStandardJsonInputLanguage::Yul => None,
        };

        let last_contract = Self::get_last_contract(self.language, &solc_output, &sources)?;

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
                let build = hex::decode(bytecode_string).map_err(|error| {
                    anyhow::anyhow!("EVM bytecode of the contract `{path}` is invalid: {error}")
                })?;
                builds.insert(path, build);
            }
        }

        Ok(EVMInput::new(builds, method_identifiers, last_contract))
    }

    fn all_modes(&self, _target: era_compiler_common::Target) -> Vec<Mode> {
        match (self.language, self.toolchain) {
            (SolcStandardJsonInputLanguage::Solidity, Toolchain::SolcLLVM) => {
                SOLIDITY_MLIR_MODES.clone()
            }
            (SolcStandardJsonInputLanguage::Solidity, _) => SOLIDITY_MODES.clone(),
            (SolcStandardJsonInputLanguage::Yul, Toolchain::SolcLLVM) => YUL_MLIR_MODES.clone(),
            (SolcStandardJsonInputLanguage::Yul, _) => YUL_MODES.clone(),
        }
    }

    fn allows_multi_contract_files(&self) -> bool {
        true
    }
}
