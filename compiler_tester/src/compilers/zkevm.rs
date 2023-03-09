//!
//! The zkEVM compiler.
//!

use std::collections::BTreeMap;
use std::collections::HashMap;

use super::build::Build as zkEVMContractBuild;
use super::mode::zkevm::Mode as ZKEVMMode;
use super::mode::Mode;
use super::Compiler;

///
/// The zkEVM compiler.
///
#[allow(non_camel_case_types)]
pub struct zkEVMCompiler {
    /// The name-to-code source files mapping.
    sources: Vec<(String, String)>,
}

impl Compiler for zkEVMCompiler {
    fn new(
        sources: Vec<(String, String)>,
        _libraries: BTreeMap<String, BTreeMap<String, String>>,
        _debug_config: Option<compiler_llvm_context::DebugConfig>,
        _is_system_mode: bool,
    ) -> Self {
        Self { sources }
    }

    fn modes() -> Vec<Mode> {
        vec![ZKEVMMode::default().into()]
    }

    fn compile(
        &self,
        _mode: &Mode,
        _is_system_contract_mode: bool,
    ) -> anyhow::Result<HashMap<String, zkEVMContractBuild>> {
        self.sources
            .iter()
            .map(|(path, source_code)| {
                zkevm_assembly::Assembly::try_from(source_code.to_owned())
                    .map_err(anyhow::Error::new)
                    .and_then(zkEVMContractBuild::new)
                    .map(|build| (path.to_string(), build))
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
