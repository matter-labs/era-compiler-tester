//!
//! The LLVM compiler.
//!

pub mod mode;

use std::collections::BTreeMap;
use std::collections::HashMap;

use sha3::Digest;

use crate::compilers::mode::Mode;
use crate::compilers::Compiler;
use crate::vm::eravm::input::build::Build as EraVMBuild;
use crate::vm::eravm::input::Input as EraVMInput;
use crate::vm::evm::input::build::Build as EVMBuild;
use crate::vm::evm::input::Input as EVMInput;

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
        _libraries: BTreeMap<String, BTreeMap<String, String>>,
        mode: &Mode,
        debug_config: Option<era_compiler_llvm_context::DebugConfig>,
    ) -> anyhow::Result<EraVMInput> {
        let mode = LLVMMode::unwrap(mode);

        let last_contract = sources
            .last()
            .ok_or_else(|| anyhow::anyhow!("LLVM IR sources are empty"))?
            .0
            .clone();

        let project = era_compiler_solidity::Project::try_from_llvm_ir_sources(sources.into_iter().collect())?;

        let builds = project
            .compile_to_eravm(
                mode.llvm_optimizer_settings.to_owned(),
                true,
                true,
                zkevm_assembly::get_encoding_mode(),
                debug_config.clone(),
            )?
            .contracts
            .into_iter()
            .map(|(path, contract)| {
                let assembly = zkevm_assembly::Assembly::from_string(
                    contract.build.assembly_text,
                    contract.build.metadata_hash,
                )
                .map_err(anyhow::Error::new)?;

                let build = EraVMBuild::new_with_hash(assembly, contract.build.bytecode_hash)?;
                Ok((path, build))
            })
            .collect::<anyhow::Result<HashMap<String, EraVMBuild>>>()?;

        Ok(EraVMInput::new(builds, None, last_contract))
    }

    fn compile_for_evm(
        &self,
        _test_path: String,
        sources: Vec<(String, String)>,
        _libraries: BTreeMap<String, BTreeMap<String, String>>,
        mode: &Mode,
        debug_config: Option<era_compiler_llvm_context::DebugConfig>,
    ) -> anyhow::Result<EVMInput> {
        let mode = LLVMMode::unwrap(mode);

        let last_contract = sources
            .last()
            .ok_or_else(|| anyhow::anyhow!("LLVM IR sources are empty"))?
            .0
            .clone();

        let project = era_compiler_solidity::Project::try_from_llvm_ir_sources(sources.into_iter().collect())?;

        let builds = project
            .compile_to_evm(
                mode.llvm_optimizer_settings.to_owned(),
                true,
                debug_config.clone(),
            )?
            .contracts
            .into_iter()
            .map(|(path, contract)| {
                let build = EVMBuild::new(era_compiler_llvm_context::EVMBuild::default(), contract.runtime_build);
                Ok((path, build))
            })
            .collect::<anyhow::Result<HashMap<String, EVMBuild>>>()?;

        Ok(EVMInput::new(builds, None, last_contract))
    }

    fn all_modes(&self) -> Vec<Mode> {
        MODES.clone()
    }

    fn allows_multi_contract_files(&self) -> bool {
        false
    }
}
