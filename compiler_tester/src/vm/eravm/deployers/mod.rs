//!
//! The contract deployers.
//!

pub mod dummy_deployer;
pub mod system_contract_deployer;

use crate::vm::eravm::EraVM;
use crate::vm::execution_result::ExecutionResult;

///
/// The deployer trait.
///
pub trait EraVMDeployer {
    ///
    /// Create new deployer instance.
    ///
    fn new() -> Self;

    ///
    /// Deploy an EraVM contract.
    ///
    fn deploy_eravm<const M: bool>(
        &mut self,
        test_name: String,
        caller: web3::types::Address,
        bytecode_hash: web3::types::U256,
        constructor_calldata: Vec<u8>,
        value: Option<u128>,
        vm: &mut EraVM,
    ) -> anyhow::Result<ExecutionResult>;

    ///
    /// Deploy an EVM contract to be run on the interpreter.
    ///
    fn deploy_evm<const M: bool>(
        &mut self,
        test_name: String,
        caller: web3::types::Address,
        deploy_code: Vec<u8>,
        constructor_calldata: Vec<u8>,
        value: Option<u128>,
        vm: &mut EraVM,
    ) -> anyhow::Result<ExecutionResult>;
}
