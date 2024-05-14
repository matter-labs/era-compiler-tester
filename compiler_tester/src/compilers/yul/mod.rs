//!
//! The Yul compiler.
//!

pub mod mode;

use std::collections::BTreeMap;
use std::collections::HashMap;
use std::path::PathBuf;

use crate::compilers::mode::Mode;
use crate::compilers::solidity::SolidityCompiler;
use crate::compilers::Compiler;
use crate::vm::eravm::input::build::Build as EraVMBuild;
use crate::vm::eravm::input::Input as EraVMInput;
use crate::vm::evm::input::build::Build as EVMBuild;
use crate::vm::evm::input::Input as EVMInput;

use self::mode::Mode as YulMode;

///
/// The Yul compiler.
///
#[derive(Default)]
pub struct YulCompiler;

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

impl Compiler for YulCompiler {
    fn compile_for_eravm(
        &self,
        _test_path: String,
        sources: Vec<(String, String)>,
        _libraries: BTreeMap<String, BTreeMap<String, String>>,
        mode: &Mode,
        debug_config: Option<era_compiler_llvm_context::DebugConfig>,
    ) -> anyhow::Result<EraVMInput> {
        let mode = YulMode::unwrap(mode);

        let solc_validator = if mode.enable_eravm_extensions {
            None
        } else {
            Some(SolidityCompiler::executable(
                &era_compiler_solidity::SolcCompiler::LAST_SUPPORTED_VERSION,
            )?)
        };

        let last_contract = sources
            .last()
            .ok_or_else(|| anyhow::anyhow!("Yul sources are empty"))?
            .0
            .clone();

        let builds = sources
            .into_iter()
            .map(|(path, source)| {
                let project = era_compiler_solidity::Project::try_from_yul_string(
                    PathBuf::from(path.as_str()).as_path(),
                    source.as_str(),
                    solc_validator.as_ref(),
                )?;

                let contract = project
                    .compile_to_eravm(
                        mode.llvm_optimizer_settings.to_owned(),
                        mode.enable_eravm_extensions,
                        true,
                        zkevm_assembly::get_encoding_mode(),
                        debug_config.clone(),
                    )?
                    .contracts
                    .remove(path.as_str())
                    .ok_or_else(|| {
                        anyhow::anyhow!("Contract `{}` not found in the Yul project", path)
                    })?;

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
        let mode = YulMode::unwrap(mode);

        let solc_validator = Some(SolidityCompiler::executable(
            &era_compiler_solidity::SolcCompiler::LAST_SUPPORTED_VERSION,
        )?);

        let last_contract = sources
            .last()
            .ok_or_else(|| anyhow::anyhow!("Yul sources are empty"))?
            .0
            .clone();

        let builds = sources
            .into_iter()
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
                    .remove(path.as_str())
                    .ok_or_else(|| {
                        anyhow::anyhow!("Contract `{}` not found in the Yul project", path)
                    })?;

                let build = EVMBuild::new(contract.deploy_build, contract.runtime_build);
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
