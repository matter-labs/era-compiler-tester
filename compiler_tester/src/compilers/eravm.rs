//!
//! The EraVM compiler.
//!

use std::collections::BTreeMap;
use std::collections::HashMap;

use super::mode::eravm::Mode as EraVMMode;
use super::mode::Mode;
use super::output::build::Build as EraVMContractBuild;
use super::output::Output;
use super::Compiler;

///
/// The EraVM compiler.
///
#[allow(non_camel_case_types)]
pub struct EraVMCompiler;

impl EraVMCompiler {
    ///
    /// A shortcut constructor.
    ///
    pub fn new() -> Self {
        Self
    }
}

impl Compiler for EraVMCompiler {
    fn modes(&self) -> Vec<Mode> {
        vec![EraVMMode::default().into()]
    }

    fn compile(
        &self,
        _test_path: String,
        sources: Vec<(String, String)>,
        _libraries: BTreeMap<String, BTreeMap<String, String>>,
        _mode: &Mode,
        _is_system_mode: bool,
        _is_system_contracts_mode: bool,
        _debug_config: Option<compiler_llvm_context::DebugConfig>,
    ) -> anyhow::Result<Output> {
        let builds = sources
            .iter()
            .map(|(path, source_code)| {
                zkevm_assembly::Assembly::try_from(source_code.to_owned())
                    .map_err(anyhow::Error::new)
                    .and_then(EraVMContractBuild::new)
                    .map(|build| (path.to_string(), build))
            })
            .collect::<anyhow::Result<HashMap<String, EraVMContractBuild>>>()?;

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
