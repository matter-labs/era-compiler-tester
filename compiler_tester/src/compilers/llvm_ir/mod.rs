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
            era_compiler_common::EraVMMetadataHashType::IPFS,
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
            Self::Zksolc => {
                let linker_symbols = libraries.as_linker_symbols()?;

                let sources = sources
                    .into_iter()
                    .map(|(path, source)| (path, era_solc::StandardJsonInputSource::from(source)))
                    .collect::<BTreeMap<_, _>>();

                let project = era_compiler_solidity::Project::try_from_llvm_ir_sources(
                    sources, libraries, None,
                )?;

                let build = project.compile_to_evm(
                    &mut vec![],
                    era_compiler_common::EVMMetadataHashType::IPFS,
                    true,
                    llvm_ir_mode.llvm_optimizer_settings.to_owned(),
                    llvm_options,
                    debug_config.clone(),
                )?;
                build.check_errors()?;

                let build = build.link(
                    linker_symbols,
                    Some(vec![("zksolc".to_owned(), semver::Version::new(0, 0, 0))]),
                );
                build.check_errors()?;

                let builds: HashMap<String, Vec<u8>> = build
                    .results
                    .into_iter()
                    .map(|(path, result)| {
                        (path, result.expect("Always valid").deploy_object.bytecode)
                    })
                    .collect();
                builds
            }
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
                    solx_standard_json::InputSelection::new(selectors),
                    solx_standard_json::InputMetadata::default(),
                    vec![],
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
                            .as_str();
                        let build = hex::decode(bytecode_string).expect("Always valid");
                        builds.insert(name, build);
                    }
                }
                builds
            }
        };

        Ok(EVMInput::new(builds, None, last_contract))
    }

    fn all_modes(&self, target: era_compiler_common::Target) -> Vec<Mode> {
        era_compiler_llvm_context::OptimizerSettings::combinations(target)
            .into_iter()
            .map(|llvm_optimizer_settings| LLVMMode::new(llvm_optimizer_settings).into())
            .collect::<Vec<Mode>>()
    }

    fn allows_multi_contract_files(&self) -> bool {
        false
    }
}
