//!
//! The Yul compiler.
//!

use std::collections::BTreeMap;
use std::collections::HashMap;
use std::path::PathBuf;

use super::mode::yul::Mode as YulMode;
use super::mode::Mode;
use super::solidity::SolidityCompiler;
use super::Compiler;
use crate::vm::eravm::input::build::Build as EraVMBuild;
use crate::vm::eravm::input::Input as EraVMInput;
use crate::vm::evm::input::build::Build as EVMBuild;
use crate::vm::evm::input::Input as EVMInput;

///
/// The Yul compiler.
///
#[derive(Default)]
pub struct YulCompiler;

lazy_static::lazy_static! {
    ///
    /// The Yul compiler supported modes.
    ///
    static ref MODES: Vec<Mode> = {
        era_compiler_llvm_context::OptimizerSettings::combinations()
            .into_iter()
            .map(|llvm_optimizer_settings| YulMode::new(llvm_optimizer_settings).into())
            .collect::<Vec<Mode>>()
    };
}

impl YulCompiler {
    ///
    /// A shortcut constructor.
    ///
    pub fn new() -> Self {
        Self::default()
    }
}

impl Compiler for YulCompiler {
    fn compile_for_eravm(
        &self,
        _test_path: String,
        sources: Vec<(String, String)>,
        _libraries: BTreeMap<String, BTreeMap<String, String>>,
        mode: &Mode,
        is_system_mode: bool,
        _is_system_contracts_mode: bool,
        debug_config: Option<era_compiler_llvm_context::DebugConfig>,
    ) -> anyhow::Result<EraVMInput> {
        let mode = YulMode::unwrap(mode);

        let solc_validator = if is_system_mode {
            None
        } else {
            Some(SolidityCompiler::get_system_contract_solc()?)
        };

        let builds = sources
            .iter()
            .map(|(path, source)| {
                let project = era_compiler_solidity::Project::try_from_yul_string(
                    PathBuf::from(path.as_str()).as_path(),
                    source.as_str(),
                    solc_validator.as_ref(),
                )?;

                let contract = project
                    .compile_to_eravm(
                        mode.llvm_optimizer_settings.to_owned(),
                        is_system_mode,
                        true,
                        zkevm_assembly::get_encoding_mode(),
                        debug_config.clone(),
                    )?
                    .contracts
                    .remove(path)
                    .ok_or_else(|| {
                        anyhow::anyhow!("Contract `{}` not found in yul project", path)
                    })?;
                let assembly = zkevm_assembly::Assembly::from_string(
                    contract.build.assembly_text,
                    contract.build.metadata_hash,
                )
                .expect("Always valid");

                let build = EraVMBuild::new_with_hash(assembly, contract.build.bytecode_hash)?;
                Ok((path.to_owned(), build))
            })
            .collect::<anyhow::Result<HashMap<String, EraVMBuild>>>()?;

        let last_contract = sources
            .last()
            .ok_or_else(|| anyhow::anyhow!("Sources is empty"))?
            .0
            .clone();

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
        let mode = YulMode::unwrap(mode);

        let solc_validator = Some(SolidityCompiler::get_system_contract_solc()?);

        let builds = sources
            .iter()
            .map(|(path, source)| {
                let project = era_compiler_solidity::Project::try_from_yul_string(
                    PathBuf::from(path.as_str()).as_path(),
                    source.as_str(),
                    solc_validator.as_ref(),
                )?;

                let contract = project
                    .compile_to_evm(
                        mode.llvm_optimizer_settings.to_owned(),
                        true,
                        debug_config.clone(),
                    )?
                    .contracts
                    .remove(path)
                    .ok_or_else(|| {
                        anyhow::anyhow!("Contract `{}` not found in yul project", path)
                    })?;

                let build = EVMBuild::new(contract.deploy_build, contract.runtime_build);
                Ok((path.to_owned(), build))
            })
            .collect::<anyhow::Result<HashMap<String, EVMBuild>>>()?;

        let last_contract = sources
            .last()
            .ok_or_else(|| anyhow::anyhow!("Sources is empty"))?
            .0
            .clone();

        Ok(EVMInput::new(builds, None, last_contract))
    }

    fn modes(&self) -> Vec<Mode> {
        MODES.clone()
    }

    fn has_multiple_contracts(&self) -> bool {
        false
    }
}
