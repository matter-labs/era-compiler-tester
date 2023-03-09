//!
//! The LLVM compiler.
//!

use std::collections::BTreeMap;
use std::collections::HashMap;

use super::build::Build as zkEVMContractBuild;
use super::mode::llvm::Mode as LLVMMode;
use super::mode::Mode;
use super::Compiler;

///
/// The LLVM compiler.
///
pub struct LLVMCompiler {
    /// The name-to-code source files mapping.
    sources: Vec<(String, String)>,
    /// The compiler debug config.
    debug_config: Option<compiler_llvm_context::DebugConfig>,
}

lazy_static::lazy_static! {
    ///
    /// The LLVM compiler supported modes.
    ///
    static ref MODES: Vec<Mode> = {
        compiler_llvm_context::OptimizerSettings::combinations()
            .into_iter()
            .map(|llvm_optimizer_settings| LLVMMode::new(llvm_optimizer_settings).into())
            .collect::<Vec<Mode>>()
    };
}

impl LLVMCompiler {
    ///
    /// Compiles the source.
    ///
    fn compile_source(
        &self,
        source_code: &str,
        name: &str,
        mode: &LLVMMode,
    ) -> anyhow::Result<zkEVMContractBuild> {
        let target_machine =
            compiler_llvm_context::TargetMachine::new(&mode.llvm_optimizer_settings)?;

        let llvm = inkwell::context::Context::create();
        let memory_buffer = inkwell::memory_buffer::MemoryBuffer::create_from_memory_range_copy(
            source_code.as_bytes(),
            name,
        );
        let module = llvm
            .create_module_from_ir(memory_buffer)
            .map_err(|error| anyhow::anyhow!(error.to_string()))?;

        if let Some(ref debug_config) = self.debug_config {
            debug_config.dump_llvm_ir_unoptimized(name, &module)?;
        }
        module
            .verify()
            .map_err(|error| anyhow::anyhow!("Unoptimized LLVM IR verification: {}", error))?;

        let optimizer = compiler_llvm_context::Optimizer::new(
            target_machine,
            mode.llvm_optimizer_settings.clone(),
        );
        optimizer
            .run(&module)
            .map_err(|error| anyhow::anyhow!(error.to_string()))?;
        if let Some(ref debug_config) = self.debug_config {
            debug_config.dump_llvm_ir_optimized(name, &module)?;
        }
        module
            .verify()
            .map_err(|error| anyhow::anyhow!("Optimized LLVM IR verification: {}", error))?;

        let assembly_text = match optimizer.target_machine().write_to_memory_buffer(&module) {
            Ok(assembly) => String::from_utf8_lossy(assembly.as_slice()).to_string(),
            Err(error) => {
                anyhow::bail!("LLVM module compiling: {}", error);
            }
        };

        if let Some(ref debug_config) = self.debug_config {
            debug_config.dump_assembly(name, assembly_text.as_str())?;
        }

        let assembly = zkevm_assembly::Assembly::try_from(assembly_text)
            .map_err(|error| anyhow::anyhow!(error))?;

        zkEVMContractBuild::new(assembly)
    }
}

impl Compiler for LLVMCompiler {
    fn new(
        sources: Vec<(String, String)>,
        _libraries: BTreeMap<String, BTreeMap<String, String>>,
        debug_config: Option<compiler_llvm_context::DebugConfig>,
        _is_system_mode: bool,
    ) -> Self {
        Self {
            sources,
            debug_config,
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
        let mode = LLVMMode::unwrap(mode);
        self.sources
            .iter()
            .map(|(path, source)| {
                self.compile_source(source, path, mode)
                    .map(|build| (path.to_owned(), build))
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
