//!
//! The Yul compiler.
//!

use std::collections::BTreeMap;
use std::collections::HashMap;

use super::build::Build as zkEVMContractBuild;
use super::mode::yul::Mode as YulMode;
use super::mode::Mode;
use super::Compiler;

///
/// The Yul compiler.
///
pub struct YulCompiler {
    /// The name-to-code source files mapping.
    sources: Vec<(String, String)>,
    /// The compiler debug config.
    debug_config: Option<compiler_llvm_context::DebugConfig>,
    /// The compiler system mode flag.
    is_system_mode: bool,
}

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

impl Compiler for YulCompiler {
    fn new(
        sources: Vec<(String, String)>,
        _libraries: BTreeMap<String, BTreeMap<String, String>>,
        debug_config: Option<compiler_llvm_context::DebugConfig>,
        is_system_mode: bool,
    ) -> Self {
        Self {
            sources,
            debug_config,
            is_system_mode,
        }
    }

    fn modes() -> Vec<Mode> {
        MODES.clone()
    }

    fn compile(
        &self,
        mode: &Mode,
        _is_system_contract_mode: bool,
    ) -> anyhow::Result<HashMap<String, zkEVMContractBuild>> {
        let mode = YulMode::unwrap(mode);

        self.sources
            .iter()
            .map(|(path, source)| {
                let project = compiler_solidity::Project::try_from_yul_string(
                    path.as_str(),
                    source.as_str(),
                )?;

                let target_machine =
                    compiler_llvm_context::TargetMachine::new(&mode.llvm_optimizer_settings)?;
                let contract = project
                    .compile_all(
                        target_machine,
                        mode.llvm_optimizer_settings.clone(),
                        self.is_system_mode,
                        true,
                        self.debug_config.clone(),
                    )?
                    .contracts
                    .remove(path)
                    .ok_or_else(|| {
                        anyhow::anyhow!("Contract `{}` not found in yul project", path)
                    })?;

                let build = zkEVMContractBuild::new_with_hash(
                    contract.build.assembly,
                    contract.build.bytecode_hash,
                )?;
                Ok((path.to_owned(), build))
            })
            .collect()
    }

    fn last_contract(
        &self,
        _mode: &Mode,
        _is_system_contract_mode: bool,
    ) -> anyhow::Result<String> {
        Ok(self
            .sources
            .last()
            .ok_or_else(|| anyhow::anyhow!("Sources is empty"))?
            .0
            .clone())
    }

    fn has_many_contracts() -> bool {
        false
    }

    fn check_pragmas(&self, _mode: &Mode) -> bool {
        true
    }

    fn check_ethereum_tests_params(_mode: &Mode, _params: &solidity_adapter::Params) -> bool {
        true
    }
}
