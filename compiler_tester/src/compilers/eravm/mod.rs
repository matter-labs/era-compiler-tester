//!
//! The EraVM compiler.
//!

pub mod mode;

use std::collections::BTreeMap;
use std::collections::HashMap;

use era_compiler_solidity::CollectableError;

use crate::compilers::mode::Mode;
use crate::compilers::Compiler;
use crate::vm::eravm::input::Input as EraVMInput;
use crate::vm::evm::input::Input as EVMInput;

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
        _libraries: BTreeMap<String, BTreeMap<String, String>>,
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
                .map(|(path, source)| {
                    (
                        path,
                        era_compiler_solidity::SolcStandardJsonInputSource::from(source),
                    )
                })
                .collect(),
            None,
        )?;

        let build = project.compile_to_eravm(
            &mut vec![],
            true,
            true,
            era_compiler_llvm_context::OptimizerSettings::none(),
            llvm_options,
            true,
            None,
            debug_config.clone(),
        )?;
        build.collect_errors()?;
        let builds = build
            .contracts
            .into_iter()
            .map(|(path, result)| Ok((path, result.expect("Always valid").build)))
            .collect::<anyhow::Result<HashMap<String, era_compiler_llvm_context::EraVMBuild>>>()?;

        Ok(EraVMInput::new(builds, None, last_contract))
    }

    fn compile_for_evm(
        &self,
        _test_path: String,
        _sources: Vec<(String, String)>,
        _libraries: BTreeMap<String, BTreeMap<String, String>>,
        _mode: &Mode,
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
