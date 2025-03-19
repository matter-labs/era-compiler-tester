//!
//! The LLVM compiler.
//!

pub mod mode;

use std::collections::HashMap;

use era_solc::CollectableError;

use crate::compilers::mode::Mode;
use crate::compilers::Compiler;
use crate::vm::eravm::input::Input as EraVMInput;
use crate::vm::revm::input::Input as EVMInput;

use self::mode::Mode as LLVMMode;

///
/// The LLVM compiler.
///
#[derive(Default)]
pub struct LLVMCompiler;

lazy_static::lazy_static! {
    ///
    /// All supported modes.
    ///
    static ref MODES: Vec<Mode> = {
        era_compiler_llvm_context::OptimizerSettings::combinations()
            .into_iter()
            .map(|llvm_optimizer_settings| LLVMMode::new(llvm_optimizer_settings).into())
            .collect::<Vec<Mode>>()
    };
}

impl Compiler for LLVMCompiler {
    fn compile_for_eravm(
        &self,
        _test_path: String,
        sources: Vec<(String, String)>,
        libraries: era_solc::StandardJsonInputLibraries,
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
            .into_iter()
            .map(|(path, result)| Ok((path, result.expect("Always valid").build)))
            .collect::<anyhow::Result<HashMap<String, era_compiler_llvm_context::EraVMBuild>>>()?;

        Ok(EraVMInput::new(builds, None, last_contract))
    }

    fn compile_for_evm(
        &self,
        _test_path: String,
        sources: Vec<(String, String)>,
        _libraries: era_solc::StandardJsonInputLibraries,
        mode: &Mode,
        _test_params: Option<&solidity_adapter::Params>,
        llvm_options: Vec<String>,
        debug_config: Option<era_compiler_llvm_context::DebugConfig>,
    ) -> anyhow::Result<EVMInput> {
        let mode = LLVMMode::unwrap(mode);

        let last_contract = sources
            .last()
            .ok_or_else(|| anyhow::anyhow!("LLVM IR sources are empty"))?
            .0
            .clone();

        let project = era_compiler_solidity::Project::try_from_llvm_ir_sources(
            sources
                .into_iter()
                .map(|(path, source)| (path, era_solc::StandardJsonInputSource::from(source)))
                .collect(),
            era_solc::StandardJsonInputLibraries::default(),
            None,
        )?;

        let build = project.compile_to_evm(
            &mut vec![],
            era_compiler_common::HashType::Ipfs,
            mode.llvm_optimizer_settings.to_owned(),
            llvm_options,
            debug_config.clone(),
        )?;
        build.check_errors()?;
        let builds: HashMap<String, Vec<u8>> = build
            .results
            .into_iter()
            .map(|(path, result)| (path, result.expect("Always valid").deploy_object.bytecode))
            .collect();

        Ok(EVMInput::new(builds, None, last_contract))
    }

    fn all_modes(&self) -> Vec<Mode> {
        MODES.clone()
    }

    fn allows_multi_contract_files(&self) -> bool {
        false
    }
}
