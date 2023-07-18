//!
//! The contract compilers for different languages.
//!

pub mod cache;
pub mod downloader;
pub mod llvm;
pub mod mode;
pub mod output;
pub mod solidity;
pub mod vyper;
pub mod yul;
pub mod zkevm;

use std::collections::BTreeMap;

use self::mode::Mode;
use self::output::Output;

///
/// The compiler trait.
///
pub trait Compiler: Send + Sync + 'static {
    ///
    /// Returns supported compiler modes.
    ///
    fn modes(&self) -> Vec<Mode>;

    ///
    /// Compile all the sources.
    ///
    #[allow(clippy::too_many_arguments)]
    fn compile(
        &self,
        test_path: String,
        sources: Vec<(String, String)>,
        libraries: BTreeMap<String, BTreeMap<String, String>>,
        mode: &Mode,
        is_system_mode: bool,
        is_system_contracts_mode: bool,
        debug_config: Option<compiler_llvm_context::DebugConfig>,
    ) -> anyhow::Result<Output>;

    ///
    /// Returns true if the one source file can contains many contracts, false otherwise.
    ///
    fn has_many_contracts(&self) -> bool;
}
