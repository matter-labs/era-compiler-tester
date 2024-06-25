//!
//! The contract compilers for different languages.
//!

pub mod cache;
pub mod downloader;
pub mod eravm;
pub mod llvm;
pub mod mode;
pub mod solidity;
pub mod vyper;
pub mod yul;

use std::collections::BTreeMap;

use crate::vm::eravm::input::Input as EraVMInput;
use crate::vm::evm::input::Input as EVMInput;

use self::mode::Mode;

///
/// The compiler trait.
///
pub trait Compiler: Send + Sync + 'static {
    ///
    /// Compile all sources for EraVM.
    ///
    fn compile_for_eravm(
        &self,
        test_path: String,
        sources: Vec<(String, String)>,
        libraries: BTreeMap<String, BTreeMap<String, String>>,
        mode: &Mode,
        llvm_options: Vec<String>,
        debug_config: Option<era_compiler_llvm_context::DebugConfig>,
    ) -> anyhow::Result<EraVMInput>;

    ///
    /// Compile all sources for EVM.
    ///
    fn compile_for_evm(
        &self,
        test_path: String,
        sources: Vec<(String, String)>,
        libraries: BTreeMap<String, BTreeMap<String, String>>,
        mode: &Mode,
        test_params: Option<&solidity_adapter::Params>,
        llvm_options: Vec<String>,
        debug_config: Option<era_compiler_llvm_context::DebugConfig>,
    ) -> anyhow::Result<EVMInput>;

    ///
    /// Returns all supported combinations of compiler settings.
    ///
    fn all_modes(&self) -> Vec<Mode>;

    ///
    /// Whether one source file can contains multiple contracts.
    ///
    fn allows_multi_contract_files(&self) -> bool;
}
