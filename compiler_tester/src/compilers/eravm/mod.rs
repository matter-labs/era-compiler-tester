//!
//! The EraVM compiler.
//!

pub mod mode;

use std::collections::HashMap;

use era_solc::CollectableError;

use crate::compilers::mode::Mode;
use crate::compilers::Compiler;
use crate::vm::eravm::input::Input as EraVMInput;
use crate::vm::revm::input::Input as EVMInput;

use self::mode::Mode as EraVMMode;

///
/// The EraVM compiler.
///
#[derive(Default)]
pub struct EraVMCompiler;

impl Compiler for EraVMCompiler {
    fn compile_for_eravm(
        &self,
        _test_path: String,
        sources: Vec<(String, String)>,
        libraries: era_solc::StandardJsonInputLibraries,
        _mode: &Mode,
        llvm_options: Vec<String>,
        debug_config: Option<era_compiler_llvm_context::DebugConfig>,
    ) -> anyhow::Result<EraVMInput> {
        let last_contract = sources
            .last()
            .ok_or_else(|| anyhow::anyhow!("EraVM sources are empty"))?
            .0
            .clone();

        let project = era_compiler_solidity::Project::try_from_eravm_assembly_sources(
            sources
                .into_iter()
                .map(|(path, source)| (path, era_solc::StandardJsonInputSource::from(source)))
                .collect(),
            None,
        )?;

        let build = project.compile_to_eravm(
            &mut vec![],
            true,
            era_compiler_common::HashType::Ipfs,
            era_compiler_llvm_context::OptimizerSettings::none(),
            llvm_options,
            true,
            debug_config.clone(),
        )?;
        build.check_errors()?;
        let build = build.link(libraries.as_linker_symbols()?);
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
        _sources: Vec<(String, String)>,
        _libraries: era_solc::StandardJsonInputLibraries,
        _mode: &Mode,
        _test_params: Option<&solidity_adapter::Params>,
        _llvm_options: Vec<String>,
        _debug_config: Option<era_compiler_llvm_context::DebugConfig>,
    ) -> anyhow::Result<EVMInput> {
        anyhow::bail!("EraVM assembly cannot be compiled to EVM");
    }

    fn all_modes(&self) -> Vec<Mode> {
        vec![EraVMMode::default().into()]
    }

    fn allows_multi_contract_files(&self) -> bool {
        false
    }
}
