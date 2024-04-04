//!
//! The LLVM compiler.
//!

pub mod mode;

use std::collections::BTreeMap;
use std::collections::HashMap;

use sha3::Digest;

use crate::compilers::mode::Mode;
use crate::compilers::Compiler;
use crate::vm::eravm::input::build::Build as EraVMBuild;
use crate::vm::eravm::input::Input as EraVMInput;
use crate::vm::evm::input::build::Build as EVMBuild;
use crate::vm::evm::input::Input as EVMInput;

use self::mode::Mode as LLVMMode;

///
/// The LLVM compiler.
///
#[derive(Default)]
pub struct LLVMCompiler;

lazy_static::lazy_static! {
    ///
    /// All supported modes.
    ///
    static ref MODES: Vec<Mode> = {
        era_compiler_llvm_context::OptimizerSettings::combinations()
            .into_iter()
            .map(|llvm_optimizer_settings| LLVMMode::new(llvm_optimizer_settings).into())
            .collect::<Vec<Mode>>()
    };
}

impl Compiler for LLVMCompiler {
    fn compile_for_eravm(
        &self,
        _test_path: String,
        sources: Vec<(String, String)>,
        _libraries: BTreeMap<String, BTreeMap<String, String>>,
        mode: &Mode,
        debug_config: Option<era_compiler_llvm_context::DebugConfig>,
    ) -> anyhow::Result<EraVMInput> {
        let mode = LLVMMode::unwrap(mode);

        let last_contract = sources
            .last()
            .ok_or_else(|| anyhow::anyhow!("LLVM IR sources are empty"))?
            .0
            .clone();

        let builds = sources
            .into_iter()
            .map(|(path, source)| {
                let llvm = inkwell::context::Context::create();
                let memory_buffer =
                    inkwell::memory_buffer::MemoryBuffer::create_from_memory_range_copy(
                        source.as_bytes(),
                        path.as_str(),
                    );
                let module = llvm
                    .create_module_from_ir(memory_buffer)
                    .map_err(|error| anyhow::anyhow!(error.to_string()))?;
                let optimizer =
                    era_compiler_llvm_context::Optimizer::new(mode.llvm_optimizer_settings.clone());
                let source_hash = sha3::Keccak256::digest(source.as_bytes()).into();

                let context = era_compiler_llvm_context::EraVMContext::<
                    era_compiler_llvm_context::EraVMDummyDependency,
                >::new(
                    &llvm, module, optimizer, None, true, debug_config.clone()
                );
                let build = context.build(path.as_str(), Some(source_hash))?;
                let assembly = zkevm_assembly::Assembly::from_string(
                    build.assembly_text,
                    build.metadata_hash,
                )?;
                let build = EraVMBuild::new(assembly)?;

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
        debug_config: Option<era_compiler_llvm_context::DebugConfig>,
    ) -> anyhow::Result<EVMInput> {
        let mode = LLVMMode::unwrap(mode);

        let last_contract = sources
            .last()
            .ok_or_else(|| anyhow::anyhow!("LLVM IR sources are empty"))?
            .0
            .clone();

        let builds = sources
            .into_iter()
            .map(|(path, source)| {
                let optimizer =
                    era_compiler_llvm_context::Optimizer::new(mode.llvm_optimizer_settings.clone());
                let source_hash = sha3::Keccak256::digest(source.as_bytes()).into();

                let llvm = inkwell::context::Context::create();
                let memory_buffer =
                    inkwell::memory_buffer::MemoryBuffer::create_from_memory_range_copy(
                        source.as_bytes(),
                        path.as_str(),
                    );
                let module = llvm
                    .create_module_from_ir(memory_buffer)
                    .map_err(|error| anyhow::anyhow!(error.to_string()))?;
                let context = era_compiler_llvm_context::EVMContext::<
                    era_compiler_llvm_context::EVMDummyDependency,
                >::new(
                    &llvm,
                    module,
                    era_compiler_llvm_context::CodeType::Runtime,
                    optimizer,
                    None,
                    true,
                    debug_config.clone(),
                );
                let build = context.build(path.as_str(), Some(source_hash))?;

                let build = EVMBuild::new(era_compiler_llvm_context::EVMBuild::default(), build);
                Ok((path, build))
            })
            .collect::<anyhow::Result<HashMap<String, EVMBuild>>>()?;

        Ok(EVMInput::new(builds, None, last_contract))
    }

    fn all_modes(&self) -> Vec<Mode> {
        MODES.clone()
    }

    fn allows_multi_contract_files(&self) -> bool {
        false
    }
}
