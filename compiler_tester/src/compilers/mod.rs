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

use self::mode::Mode;

use crate::vm::eravm::input::Input as EraVMInput;
use crate::vm::evm::input::Input as EVMInput;

///
/// The compiler trait.
///
pub trait Compiler: Send + Sync + 'static {
    ///
    /// Compile all sources for EraVM.
    ///
    #[allow(clippy::too_many_arguments)]
    fn compile_for_eravm(
        &self,
        test_path: String,
        sources: Vec<(String, String)>,
        libraries: BTreeMap<String, BTreeMap<String, String>>,
        mode: &Mode,
        is_system_mode: bool,
        is_system_contracts_mode: bool,
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
        debug_config: Option<era_compiler_llvm_context::DebugConfig>,
    ) -> anyhow::Result<EVMInput>;

    ///
    /// Returns supported compiler modes.
    ///
    fn modes(&self) -> Vec<Mode>;

    ///
    /// Whether one source file can contains multiple contracts.
    ///
    fn has_multiple_contracts(&self) -> bool;
}
