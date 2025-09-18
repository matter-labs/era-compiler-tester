//!
//! The Yul compiler.
//!

pub mod mode;
pub mod mode_upstream;

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::collections::HashMap;
use std::sync::Arc;

use era_solc::CollectableError as ZksolcCollectableError;
use solx_standard_json::CollectableError as SolxCollectableError;

use crate::compilers::mode::Mode;
use crate::compilers::solidity::solc::compiler::standard_json::input::language::Language as SolcStandardJsonInputLanguage;
use crate::compilers::solidity::solc::SolidityCompiler as SolcCompiler;
use crate::compilers::solidity::solx::SolidityCompiler as SolxCompiler;
use crate::compilers::solidity::zksolc::SolidityCompiler as ZksolcCompiler;
use crate::compilers::Compiler;
use crate::toolchain::Toolchain;
use crate::vm::eravm::input::Input as EraVMInput;
use crate::vm::revm::input::Input as EVMInput;

use self::mode::Mode as YulMode;

///
/// The Yul compiler.
///
pub enum YulCompiler {
    /// `zksolc` toolchain.
    Zksolc,
    /// `solx` toolchain.
    Solx(Arc<SolxCompiler>),
    /// `solc` toolchain.
    Solc,
    /// `solc-llvm` toolchain.
    SolcLLVM,
}

impl Compiler for YulCompiler {
    fn compile_for_eravm(
        &self,
        _test_path: String,
        sources: Vec<(String, String)>,
        libraries: era_compiler_common::Libraries,
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
                ZksolcCompiler::LAST_ZKSYNC_SOLC_REVISION,
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
            era_compiler_common::MetadataHashType::IPFS,
            true,
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
                let mut build = era_compiler_llvm_context::EraVMBuild::new(
                    contract.build.bytecode,
                    contract.build.metadata,
                    contract.build.assembly,
                );
                build.bytecode_hash = contract.build.bytecode_hash;
                Ok((contract.name.path, build))
            })
            .collect::<anyhow::Result<HashMap<String, era_compiler_llvm_context::EraVMBuild>>>()?;

        Ok(EraVMInput::new(builds, None, last_contract))
    }

    fn compile_for_evm(
        &self,
        test_path: String,
        sources: Vec<(String, String)>,
        libraries: era_compiler_common::Libraries,
        mode: &Mode,
        test_params: Option<&solidity_adapter::Params>,
        llvm_options: Vec<String>,
        debug_config: Option<era_compiler_llvm_context::DebugConfig>,
    ) -> anyhow::Result<EVMInput> {
        match self {
            Self::Solx(solx) => {
                let yul_mode = YulMode::unwrap(mode);

                let last_contract = sources
                    .last()
                    .ok_or_else(|| anyhow::anyhow!("Yul sources are empty"))?
                    .0
                    .clone();

                let sources: BTreeMap<String, solx_standard_json::InputSource> = sources
                    .iter()
                    .map(|(path, source)| {
                        (
                            path.to_owned(),
                            solx_standard_json::InputSource::from(source.to_owned()),
                        )
                    })
                    .collect();

                let libraries = solx_utils::Libraries {
                    inner: libraries.inner,
                };

                let mut selectors = BTreeSet::new();
                selectors.insert(solx_standard_json::InputSelector::Bytecode);
                selectors.insert(solx_standard_json::InputSelector::RuntimeBytecode);
                selectors.insert(solx_standard_json::InputSelector::AST);
                selectors.insert(solx_standard_json::InputSelector::MethodIdentifiers);
                selectors.insert(solx_standard_json::InputSelector::Metadata);
                selectors.insert(solx_standard_json::InputSelector::Yul);
                let solx_input = solx_standard_json::Input::from_yul_sources(
                    sources,
                    libraries.to_owned(),
                    solx_standard_json::InputOptimizer::new(
                        yul_mode.llvm_optimizer_settings.middle_end_as_char(),
                        yul_mode.llvm_optimizer_settings.is_fallback_to_size_enabled,
                    ),
                    &solx_standard_json::InputSelection::new(selectors),
                    solx_standard_json::InputMetadata::default(),
                    llvm_options,
                );

                let solx_output = solx.standard_json(
                    mode,
                    solx_input,
                    &[],
                    debug_config
                        .as_ref()
                        .map(|debug_config| debug_config.output_directory.as_path()),
                )?;
                solx_output.check_errors()?;

                let mut builds = HashMap::with_capacity(solx_output.contracts.len());
                for (file, contracts) in solx_output.contracts.into_iter() {
                    for (_name, contract) in contracts.into_iter() {
                        let bytecode_string = contract
                            .evm
                            .as_ref()
                            .ok_or_else(|| {
                                anyhow::anyhow!("EVM object of the contract `{file}` not found")
                            })?
                            .bytecode
                            .as_ref()
                            .ok_or_else(|| {
                                anyhow::anyhow!("EVM bytecode of the contract `{file}` not found")
                            })?
                            .object
                            .as_ref()
                            .ok_or_else(|| {
                                anyhow::anyhow!(
                                    "EVM bytecode object of the contract `{file}` not found"
                                )
                            })?
                            .as_str();
                        let build = hex::decode(bytecode_string).expect("Always valid");
                        builds.insert(file.clone(), build);
                    }
                }

                Ok(EVMInput::new(builds, None, last_contract))
            }
            Self::Solc | Self::SolcLLVM => {
                let language = SolcStandardJsonInputLanguage::Yul;

                let solc_compiler = SolcCompiler::new(language, Toolchain::from(self));

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
            Self::Zksolc => unimplemented!(),
        }
    }

    fn all_modes(&self) -> Vec<Mode> {
        solx_codegen_evm::OptimizerSettings::combinations()
            .into_iter()
            .map(|llvm_optimizer_settings| {
                let llvm_optimizer_settings = era_compiler_llvm_context::OptimizerSettings::new(
                    llvm_optimizer_settings.level_middle_end,
                    match llvm_optimizer_settings.level_middle_end_size as u32 {
                        0 => era_compiler_llvm_context::OptimizerSettingsSizeLevel::Zero,
                        1 => era_compiler_llvm_context::OptimizerSettingsSizeLevel::S,
                        2 => era_compiler_llvm_context::OptimizerSettingsSizeLevel::Z,
                        _ => panic!("Invalid size level"),
                    },
                    llvm_optimizer_settings.level_back_end,
                );
                YulMode::new(llvm_optimizer_settings, false).into()
            })
            .collect::<Vec<Mode>>()
    }

    fn allows_multi_contract_files(&self) -> bool {
        false
    }
}

impl From<&YulCompiler> for Toolchain {
    fn from(value: &YulCompiler) -> Self {
        match value {
            YulCompiler::Solc => Self::Solc,
            YulCompiler::Zksolc => Self::Zksolc,
            YulCompiler::Solx { .. } => Self::IrLLVM,
            YulCompiler::SolcLLVM => Self::SolcLLVM,
        }
    }
}
