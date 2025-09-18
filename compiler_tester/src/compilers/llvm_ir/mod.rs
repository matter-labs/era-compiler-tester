//!
//! The LLVM compiler.
//!

pub mod mode;

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::collections::HashMap;
use std::sync::Arc;

use era_solc::CollectableError as ZksolcCollectableError;
use solx_standard_json::CollectableError as SolxCollectableError;

use crate::compilers::mode::Mode;
use crate::compilers::solidity::solx::SolidityCompiler as SolxCompiler;
use crate::compilers::Compiler;
use crate::vm::eravm::input::Input as EraVMInput;
use crate::vm::revm::input::Input as EVMInput;

use self::mode::Mode as LLVMMode;

///
/// The LLVM compiler.
///
pub enum LLVMIRCompiler {
    /// `zksolc` toolchain.
    Zksolc,
    /// `solx` toolchain.
    Solx(Arc<SolxCompiler>),
}

impl Compiler for LLVMIRCompiler {
    fn compile_for_eravm(
        &self,
        _test_path: String,
        sources: Vec<(String, String)>,
        libraries: era_compiler_common::Libraries,
        mode: &Mode,
        llvm_options: Vec<String>,
        debug_config: Option<era_compiler_llvm_context::DebugConfig>,
    ) -> anyhow::Result<EraVMInput> {
        let mode = LLVMMode::unwrap(mode);

        let last_contract = sources
            .last()
            .ok_or_else(|| anyhow::anyhow!("LLVM IR sources are empty"))?
            .0
            .clone();

        let linker_symbols = libraries.as_linker_symbols()?;

        let project = era_compiler_solidity::Project::try_from_llvm_ir_sources(
            sources
                .into_iter()
                .map(|(path, source)| (path, era_solc::StandardJsonInputSource::from(source)))
                .collect(),
            libraries,
            None,
        )?;

        let build = project.compile_to_eravm(
            &mut vec![],
            true,
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
            .into_iter()
            .map(|(path, result)| Ok((path, result.expect("Always valid").build)))
            .collect::<anyhow::Result<HashMap<String, era_compiler_llvm_context::EraVMBuild>>>()?;

        Ok(EraVMInput::new(builds, None, last_contract))
    }

    fn compile_for_evm(
        &self,
        _test_path: String,
        sources: Vec<(String, String)>,
        libraries: era_compiler_common::Libraries,
        mode: &Mode,
        _test_params: Option<&solidity_adapter::Params>,
        llvm_options: Vec<String>,
        debug_config: Option<era_compiler_llvm_context::DebugConfig>,
    ) -> anyhow::Result<EVMInput> {
        let llvm_ir_mode = LLVMMode::unwrap(mode);

        let last_contract = sources
            .last()
            .ok_or_else(|| anyhow::anyhow!("LLVM IR sources are empty"))?
            .0
            .clone();

        let builds = match self {
            Self::Solx(solx) => {
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
                selectors.insert(solx_standard_json::InputSelector::Metadata);
                let solx_input = solx_standard_json::Input::from_llvm_ir_sources(
                    sources,
                    libraries.to_owned(),
                    solx_standard_json::InputOptimizer::new(
                        llvm_ir_mode.llvm_optimizer_settings.middle_end_as_char(),
                        llvm_ir_mode
                            .llvm_optimizer_settings
                            .is_fallback_to_size_enabled,
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
                for (_file, contracts) in solx_output.contracts.into_iter() {
                    for (name, contract) in contracts.into_iter() {
                        let bytecode_string = contract
                            .evm
                            .as_ref()
                            .ok_or_else(|| {
                                anyhow::anyhow!("EVM object of the contract `{name}` not found")
                            })?
                            .bytecode
                            .as_ref()
                            .ok_or_else(|| {
                                anyhow::anyhow!("EVM bytecode of the contract `{name}` not found")
                            })?
                            .object
                            .as_ref()
                            .ok_or_else(|| {
                                anyhow::anyhow!(
                                    "EVM bytecode object of the contract `{name}` not found"
                                )
                            })?
                            .as_str();
                        let build = hex::decode(bytecode_string).expect("Always valid");
                        builds.insert(name, build);
                    }
                }
                builds
            }
            Self::Zksolc => unimplemented!(),
        };

        Ok(EVMInput::new(builds, None, last_contract))
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
                LLVMMode::new(llvm_optimizer_settings).into()
            })
            .collect::<Vec<Mode>>()
    }

    fn allows_multi_contract_files(&self) -> bool {
        false
    }
}
