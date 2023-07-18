//!
//! The LLVM compiler.
//!

use std::collections::BTreeMap;
use std::collections::HashMap;

use sha3::Digest;

use super::mode::llvm::Mode as LLVMMode;
use super::mode::Mode;
use super::output::build::Build as zkEVMContractBuild;
use super::output::Output;
use super::Compiler;

///
/// The LLVM compiler.
///
pub struct LLVMCompiler;

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
    /// A shortcut constructor.
    ///
    pub fn new() -> Self {
        Self
    }

    ///
    /// Compiles the source.
    ///
    fn compile_source(
        source_code: &str,
        name: &str,
        mode: &LLVMMode,
        debug_config: Option<compiler_llvm_context::DebugConfig>,
    ) -> anyhow::Result<zkEVMContractBuild> {
        let llvm = inkwell::context::Context::create();
        let memory_buffer = inkwell::memory_buffer::MemoryBuffer::create_from_memory_range_copy(
            source_code.as_bytes(),
            name,
        );
        let module = llvm
            .create_module_from_ir(memory_buffer)
            .map_err(|error| anyhow::anyhow!(error.to_string()))?;
        let optimizer = compiler_llvm_context::Optimizer::new(mode.llvm_optimizer_settings.clone());
        let source_hash = sha3::Keccak256::digest(source_code.as_bytes()).into();

        let context = compiler_llvm_context::Context::<compiler_llvm_context::DummyDependency>::new(
            &llvm,
            module,
            optimizer,
            None,
            true,
            debug_config,
        );
        let build = context.build(name, Some(source_hash))?;
        let assembly =
            zkevm_assembly::Assembly::from_string(build.assembly_text, build.metadata_hash)?;

        zkEVMContractBuild::new(assembly)
    }
}

impl Compiler for LLVMCompiler {
    fn modes(&self) -> Vec<Mode> {
        MODES.clone()
    }

    fn compile(
        &self,
        _test_path: String,
        sources: Vec<(String, String)>,
        _libraries: BTreeMap<String, BTreeMap<String, String>>,
        mode: &Mode,
        _is_system_mode: bool,
        _is_system_contracts_mode: bool,
        debug_config: Option<compiler_llvm_context::DebugConfig>,
    ) -> anyhow::Result<Output> {
        let mode = LLVMMode::unwrap(mode);

        let builds = sources
            .iter()
            .map(|(path, source)| {
                Self::compile_source(source, path, mode, debug_config.clone())
                    .map(|build| (path.to_owned(), build))
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
