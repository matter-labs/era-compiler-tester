//!
//! The EraVM compiler.
//!

pub mod mode;

use std::collections::BTreeMap;
use std::collections::HashMap;

use crate::compilers::mode::Mode;
use crate::compilers::Compiler;
use crate::vm::eravm::input::build::Build as EraVMBuild;
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
        _debug_config: Option<era_compiler_llvm_context::DebugConfig>,
    ) -> anyhow::Result<EraVMInput> {
        let last_contract = sources
            .last()
            .ok_or_else(|| anyhow::anyhow!("EraVM sources are empty"))?
            .0
            .clone();

        let builds = sources
            .into_iter()
            .map(|(path, source_code)| {
                zkevm_assembly::Assembly::try_from(source_code.to_owned())
                    .map_err(anyhow::Error::new)
                    .and_then(EraVMBuild::new)
                    .map(|build| (path, build))
            })
            .collect::<anyhow::Result<HashMap<String, EraVMBuild>>>()?;

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
        anyhow::bail!("EraVM assembly cannot be compiled to EVM");
    }

    fn all_modes(&self) -> Vec<Mode> {
        vec![EraVMMode::default().into()]
    }

    fn allows_multi_contract_files(&self) -> bool {
        false
    }
}
