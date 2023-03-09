//!
//! The contract deployers.
//!

pub mod address_predictor;
pub mod native_deployer;
pub mod system_contract_deployer;

use crate::zkevm::execution_result::ExecutionResult;
use crate::zkevm::zkEVM;

///
/// The deployer trait.
///
pub trait Deployer {
    ///
    /// Create new deployer instance.
    ///
    fn new() -> Self;

    ///
    /// Deploy a contract.
    ///
    fn deploy<const M: bool>(
        &mut self,
        test_name: String,
        caller: web3::types::Address,
        bytecode_hash: web3::types::U256,
        constructor_calldata: Vec<u8>,
        value: Option<u128>,
        vm: &mut zkEVM,
    ) -> anyhow::Result<ExecutionResult>;
}
