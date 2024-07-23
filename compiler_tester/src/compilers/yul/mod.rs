//!
//! The Yul compiler.
//!

pub mod mode;

use std::collections::BTreeMap;
use std::collections::HashMap;

use era_compiler_solidity::CollectableError;

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
        llvm_options: Vec<String>,
        debug_config: Option<era_compiler_llvm_context::DebugConfig>,
    ) -> anyhow::Result<EraVMInput> {
        let mode = YulMode::unwrap(mode);

        let solc_version = if mode.enable_eravm_extensions {
            None
        } else {
            Some(
                SolidityCompiler::executable(
                    &era_compiler_solidity::SolcCompiler::LAST_SUPPORTED_VERSION,
                )?
                .version,
            )
        };

        let last_contract = sources
            .last()
            .ok_or_else(|| anyhow::anyhow!("Yul sources are empty"))?
            .0
            .clone();

        let project = era_compiler_solidity::Project::try_from_yul_sources(
            sources
                .into_iter()
                .map(|(path, source)| {
                    (
                        path,
                        era_compiler_solidity::SolcStandardJsonInputSource::from(source),
                    )
                })
                .collect(),
            BTreeMap::new(),
            None,
            solc_version.as_ref(),
            debug_config.as_ref(),
        )?;

        let build = project.compile_to_eravm(
            &mut vec![],
            mode.enable_eravm_extensions,
            true,
            mode.llvm_optimizer_settings.to_owned(),
            llvm_options,
            true,
            None,
            debug_config.clone(),
        )?;
        build.collect_errors()?;
        let builds = build
            .contracts
            .into_iter()
            .map(|(path, build)| {
                let build = build.expect("Always valid");
                let assembly = zkevm_assembly::Assembly::from_string(
                    build.build.assembly.expect("Always exists"),
                    build.build.metadata_hash,
                )
                .map_err(anyhow::Error::new)?;

                let build = EraVMBuild::new_with_hash(assembly, build.build.bytecode_hash)?;
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
        llvm_options: Vec<String>,
        debug_config: Option<era_compiler_llvm_context::DebugConfig>,
    ) -> anyhow::Result<EVMInput> {
        let mode = YulMode::unwrap(mode);

        let solc_version = Some(
            SolidityCompiler::executable(
                &era_compiler_solidity::SolcCompiler::LAST_SUPPORTED_VERSION,
            )?
            .version,
        );

        let last_contract = sources
            .last()
            .ok_or_else(|| anyhow::anyhow!("Yul sources are empty"))?
            .0
            .clone();

        let project = era_compiler_solidity::Project::try_from_yul_sources(
            sources
                .into_iter()
                .map(|(path, source)| {
                    (
                        path,
                        era_compiler_solidity::SolcStandardJsonInputSource::from(source),
                    )
                })
                .collect(),
            BTreeMap::new(),
            None,
            solc_version.as_ref(),
            debug_config.as_ref(),
        )?;

        let build = project.compile_to_evm(
            &mut vec![],
            mode.llvm_optimizer_settings.to_owned(),
            llvm_options,
            true,
            None,
            debug_config.clone(),
        )?;
        build.collect_errors()?;
        let builds: HashMap<String, EVMBuild> = build
            .contracts
            .into_iter()
            .map(|(path, build)| {
                let build = build.expect("Always valid");
                let build = EVMBuild::new(build.deploy_build, build.runtime_build);
                (path, build)
            })
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
