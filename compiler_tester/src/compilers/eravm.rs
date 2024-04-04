//!
//! The EraVM compiler.
//!

use std::collections::BTreeMap;
use std::collections::HashMap;

use super::mode::eravm::Mode as EraVMMode;
use super::mode::Mode;
use super::Compiler;
use crate::vm::eravm::input::build::Build as EraVMBuild;
use crate::vm::eravm::input::Input as EraVMInput;
use crate::vm::evm::input::Input as EVMInput;

///
/// The EraVM compiler.
///
#[derive(Default)]
#[allow(non_camel_case_types)]
pub struct EraVMCompiler;

impl EraVMCompiler {
    ///
    /// A shortcut constructor.
    ///
    pub fn new() -> Self {
        Self::default()
    }
}

impl Compiler for EraVMCompiler {
    fn compile_for_eravm(
        &self,
        _test_path: String,
        sources: Vec<(String, String)>,
        _libraries: BTreeMap<String, BTreeMap<String, String>>,
        _mode: &Mode,
        _is_system_mode: bool,
        _is_system_contracts_mode: bool,
        _debug_config: Option<era_compiler_llvm_context::DebugConfig>,
    ) -> anyhow::Result<EraVMInput> {
        let builds = sources
            .iter()
            .map(|(path, source_code)| {
                zkevm_assembly::Assembly::try_from(source_code.to_owned())
                    .map_err(anyhow::Error::new)
                    .and_then(EraVMBuild::new)
                    .map(|build| (path.to_string(), build))
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
        _sources: Vec<(String, String)>,
        _libraries: BTreeMap<String, BTreeMap<String, String>>,
        _mode: &Mode,
        _debug_config: Option<era_compiler_llvm_context::DebugConfig>,
    ) -> anyhow::Result<EVMInput> {
        anyhow::bail!("EraVM compiler does not support EVM compilation");
    }

    fn modes(&self) -> Vec<Mode> {
        vec![EraVMMode::default().into()]
    }

    fn has_multiple_contracts(&self) -> bool {
        false
    }
}
