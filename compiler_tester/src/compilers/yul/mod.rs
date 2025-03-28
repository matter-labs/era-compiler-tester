//!
//! The Yul compiler.
//!

pub mod mode;
pub mod mode_upstream;

use std::collections::HashMap;

use era_solc::CollectableError;

use crate::compilers::mode::Mode;
use crate::compilers::solidity::upstream::solc::standard_json::input::language::Language as SolcStandardJsonInputLanguage;
use crate::compilers::solidity::upstream::SolidityCompiler as SolidityUpstreamCompiler;
use crate::compilers::solidity::SolidityCompiler;
use crate::compilers::Compiler;
use crate::toolchain::Toolchain;
use crate::vm::eravm::input::Input as EraVMInput;
use crate::vm::revm::input::Input as EVMInput;

use self::mode::Mode as YulMode;

///
/// The Yul compiler.
///
pub struct YulCompiler {
    /// The compiler toolchain to use.
    toolchain: Toolchain,
}

lazy_static::lazy_static! {
    ///
    /// All supported modes.
    ///
    static ref MODES: Vec<Mode> = {
        era_compiler_llvm_context::OptimizerSettings::combinations()
            .into_iter()
            .map(|llvm_optimizer_settings| YulMode::new(llvm_optimizer_settings, false).into())
            .collect::<Vec<Mode>>()
    };
}

impl YulCompiler {
    ///
    /// A shortcut constructor.
    ///
    pub fn new(toolchain: Toolchain) -> Self {
        Self { toolchain }
    }
}

impl Compiler for YulCompiler {
    fn compile_for_eravm(
        &self,
        _test_path: String,
        sources: Vec<(String, String)>,
        libraries: era_solc::StandardJsonInputLibraries,
        mode: &Mode,
        llvm_options: Vec<String>,
        debug_config: Option<era_compiler_llvm_context::DebugConfig>,
    ) -> anyhow::Result<EraVMInput> {
        let mode = YulMode::unwrap(mode);

        let solc_version = if mode.enable_eravm_extensions {
            None
        } else {
            Some(era_solc::Version::new(
                era_solc::Compiler::LAST_SUPPORTED_VERSION.to_string(),
                era_solc::Compiler::LAST_SUPPORTED_VERSION,
                SolidityCompiler::LAST_ZKSYNC_SOLC_REVISION,
            ))
        };

        let last_contract = sources
            .last()
            .ok_or_else(|| anyhow::anyhow!("Yul sources are empty"))?
            .0
            .clone();

        let linker_symbols = libraries.as_linker_symbols()?;

        let sources = sources
            .into_iter()
            .map(|(path, source)| (path, era_solc::StandardJsonInputSource::from(source)))
            .collect();

        let project = era_compiler_solidity::Project::try_from_yul_sources(
            sources,
            libraries,
            None,
            solc_version.as_ref(),
            debug_config.as_ref(),
        )?;

        let build = project.compile_to_eravm(
            &mut vec![],
            mode.enable_eravm_extensions,
            era_compiler_common::HashType::Ipfs,
            mode.llvm_optimizer_settings.to_owned(),
            llvm_options,
            true,
            debug_config.clone(),
        )?;
        build.check_errors()?;
        let build = build.link(linker_symbols);
        build.check_errors()?;
        let builds = build
            .results
            .into_values()
            .map(|result| {
                let contract = result.expect("Always valid");
                let build = era_compiler_llvm_context::EraVMBuild::new_with_bytecode_hash(
                    contract.build.bytecode,
                    contract.build.bytecode_hash.ok_or_else(|| {
                        anyhow::anyhow!("Bytecode hash not found in the build artifacts")
                    })?,
                    None,
                    contract.build.assembly,
                );
                Ok((contract.name.path, build))
            })
            .collect::<anyhow::Result<HashMap<String, era_compiler_llvm_context::EraVMBuild>>>()?;

        Ok(EraVMInput::new(builds, None, last_contract))
    }

    fn compile_for_evm(
        &self,
        test_path: String,
        sources: Vec<(String, String)>,
        libraries: era_solc::StandardJsonInputLibraries,
        mode: &Mode,
        test_params: Option<&solidity_adapter::Params>,
        llvm_options: Vec<String>,
        debug_config: Option<era_compiler_llvm_context::DebugConfig>,
    ) -> anyhow::Result<EVMInput> {
        match self.toolchain {
            Toolchain::Solc | Toolchain::SolcLLVM => {
                let language = SolcStandardJsonInputLanguage::Yul;

                let solc_compiler = SolidityUpstreamCompiler::new(language, self.toolchain);

                let solc_output = solc_compiler.standard_json_output_cached(
                    test_path,
                    language,
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

                let last_contract = sources
                    .last()
                    .ok_or_else(|| anyhow::anyhow!("Yul sources are empty"))?
                    .0
                    .clone();

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
                        let build = hex::decode(bytecode_string).expect("Always valid");
                        builds.insert(path, build);
                    }
                }

                Ok(EVMInput::new(builds, None, last_contract))
            }
            Toolchain::IrLLVM => {
                let mode = YulMode::unwrap(mode);

                let solc_version = era_solc::Version::new(
                    era_solc::Compiler::LAST_SUPPORTED_VERSION.to_string(),
                    era_solc::Compiler::LAST_SUPPORTED_VERSION,
                    SolidityCompiler::LAST_ZKSYNC_SOLC_REVISION,
                );

                let last_contract = sources
                    .last()
                    .ok_or_else(|| anyhow::anyhow!("Yul sources are empty"))?
                    .0
                    .clone();

                let linker_symbols = libraries.as_linker_symbols()?;

                let sources = sources
                    .into_iter()
                    .map(|(path, source)| (path, era_solc::StandardJsonInputSource::from(source)))
                    .collect();

                let project = era_compiler_solidity::Project::try_from_yul_sources(
                    sources,
                    libraries,
                    None,
                    Some(&solc_version),
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
                let builds = build
                    .results
                    .into_values()
                    .map(|result| {
                        let contract = result.expect("Always valid");
                        Ok((contract.name.path, contract.deploy_object.bytecode))
                    })
                    .collect::<anyhow::Result<HashMap<String, Vec<u8>>>>()?;

                Ok(EVMInput::new(builds, None, last_contract))
            }
        }
    }

    fn all_modes(&self) -> Vec<Mode> {
        MODES.clone()
    }

    fn allows_multi_contract_files(&self) -> bool {
        false
    }
}
