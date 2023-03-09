//!
//! The contract compilers for different languages.
//!

pub mod build;
pub mod cache;
pub mod downloader;
pub mod llvm;
pub mod mode;
pub mod solidity;
pub mod vyper;
pub mod yul;
pub mod zkevm;

use std::collections::BTreeMap;
use std::collections::HashMap;

use self::build::Build as zkEVMContractBuild;
use self::mode::Mode;

///
/// The compiler trait.
///
pub trait Compiler: Send + Sync + 'static {
    ///
    /// The constructor.
    ///
    fn new(
        sources: Vec<(String, String)>,
        libraries: BTreeMap<String, BTreeMap<String, String>>,
        debug_config: Option<compiler_llvm_context::DebugConfig>,
        is_system_mode: bool,
    ) -> Self;

    ///
    /// Returns supported compiler modes.
    ///
    fn modes() -> Vec<Mode>;

    ///
    /// Compile all the sources.
    ///
    fn compile(
        &self,
        mode: &Mode,
        is_system_contracts_mode: bool,
    ) -> anyhow::Result<HashMap<String, zkEVMContractBuild>>;

    ///
    /// Returns selector by entry.
    ///
    fn selector(
        &self,
        _mode: &Mode,
        _contract_path: &str,
        entry: &str,
        _is_system_contracts_mode: bool,
    ) -> anyhow::Result<u32> {
        u32::from_str_radix(entry, compiler_common::BASE_HEXADECIMAL)
            .map_err(|err| anyhow::anyhow!("Invalid entry value: {}", err))
    }

    ///
    /// Returns the last contract name.
    ///
    fn last_contract(&self, mode: &Mode, is_system_contracts_mode: bool) -> anyhow::Result<String>;

    ///
    /// Returns true if the one source file can contains many contracts, false otherwise.
    ///
    fn has_many_contracts() -> bool;

    ///
    /// Checks the versions in the source code pragmas.
    ///
    fn check_pragmas(&self, _mode: &Mode) -> bool;

    ///
    /// Checks the Ethereum tests params compatability.
    ///
    fn check_ethereum_tests_params(_mode: &Mode, _params: &solidity_adapter::Params) -> bool;
}
