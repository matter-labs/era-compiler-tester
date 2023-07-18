//!
//! The Yul compiler.
//!

use std::collections::BTreeMap;
use std::collections::HashMap;
use std::path::PathBuf;

use super::mode::yul::Mode as YulMode;
use super::mode::Mode;
use super::output::build::Build as zkEVMContractBuild;
use super::output::Output;
use super::solidity::SolidityCompiler;
use super::Compiler;

///
/// The Yul compiler.
///
pub struct YulCompiler;

lazy_static::lazy_static! {
    ///
    /// The Yul compiler supported modes.
    ///
    static ref MODES: Vec<Mode> = {
        compiler_llvm_context::OptimizerSettings::combinations()
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
        Self
    }
}

impl Compiler for YulCompiler {
    fn modes(&self) -> Vec<Mode> {
        MODES.clone()
    }

    fn compile(
        &self,
        _test_path: String,
        sources: Vec<(String, String)>,
        _libraries: BTreeMap<String, BTreeMap<String, String>>,
        mode: &Mode,
        is_system_mode: bool,
        _is_system_contracts_mode: bool,
        debug_config: Option<compiler_llvm_context::DebugConfig>,
    ) -> anyhow::Result<Output> {
        let mode = YulMode::unwrap(mode);

        let solc_validator = if is_system_mode {
            None
        } else {
            Some(SolidityCompiler::get_solc_by_version(
                &compiler_solidity::SolcCompiler::LAST_SUPPORTED_VERSION,
            ))
        };

        let builds = sources
            .iter()
            .map(|(path, source)| {
                let project = compiler_solidity::Project::try_from_yul_string(
                    PathBuf::from(path.as_str()).as_path(),
                    source.as_str(),
                    solc_validator.as_ref(),
                )?;

                let contract = project
                    .compile(
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

                let build =
                    zkEVMContractBuild::new_with_hash(assembly, contract.build.bytecode_hash)?;
                Ok((path.to_owned(), build))
            })
            .collect::<anyhow::Result<HashMap<String, zkEVMContractBuild>>>()?;

        let last_contract = sources
            .last()
            .ok_or_else(|| anyhow::anyhow!("Sources is empty"))?
            .0
            .clone();

        Ok(Output::new(builds, None, last_contract))
    }

    fn has_many_contracts(&self) -> bool {
        false
    }
}
