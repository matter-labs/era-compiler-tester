//!
//! The EraVM system contract deployer implementation.
//!

use crate::vm::eravm::EraVM;
use crate::vm::execution_result::ExecutionResult;

use super::Deployer;

///
/// The EraVM system contract deployer implementation.
///
#[derive(Debug, Clone)]
pub struct SystemContractDeployer;

impl SystemContractDeployer {
    /// The create method selector.
    const CREATE_METHOD_SELECTOR: u32 = 0x9c4d535b; // keccak256("create(bytes32,bytes32,bytes)")
}

impl Deployer for SystemContractDeployer {
    fn new() -> Self {
        Self
    }

    fn deploy<const M: bool>(
        &mut self,
        test_name: String,
        caller: web3::types::Address,
        bytecode_hash: web3::types::U256,
        constructor_calldata: Vec<u8>,
        value: Option<u128>,
        vm: &mut EraVM,
    ) -> anyhow::Result<ExecutionResult> {
        let mut entry_address = web3::types::Address::from_low_u64_be(
            zkevm_opcode_defs::ADDRESS_CONTRACT_DEPLOYER.into(),
        );

        let (vm_launch_option, context_u128_value) = if M {
            let mut r3 = None;
            let mut r4 = None;
            let mut r5 = None;
            if let Some(value) = value {
                let value = web3::types::U256::from(value);
                vm.mint_ether(caller, value);

                r3 = Some(value);
                r4 = Some(web3::types::U256::from(
                    zkevm_opcode_defs::ADDRESS_CONTRACT_DEPLOYER,
                ));
                r5 = Some(web3::types::U256::from(u8::from(
                    era_compiler_llvm_context::eravm_const::SYSTEM_CALL_BIT,
                )));

                entry_address = web3::types::Address::from_low_u64_be(
                    zkevm_opcode_defs::ADDRESS_MSG_VALUE.into(),
                );
            }

            let vm_launch_option =
                zkevm_tester::runners::compiler_tests::VmLaunchOption::ManualCallABI(
                    zkevm_tester::runners::compiler_tests::FullABIParams {
                        is_constructor: false,
                        is_system_call: true,
                        r3_value: r3,
                        r4_value: r4,
                        r5_value: r5,
                    },
                );
            (vm_launch_option, 0)
        } else {
            let context_u128_value = if let Some(value) = value {
                vm.mint_ether(
                    web3::types::Address::from_low_u64_be(
                        zkevm_opcode_defs::ADDRESS_CONTRACT_DEPLOYER.into(),
                    ),
                    web3::types::U256::from(value),
                );
                value
            } else {
                0
            };
            let vm_launch_option =
                zkevm_tester::runners::compiler_tests::VmLaunchOption::ManualCallABI(
                    zkevm_tester::runners::compiler_tests::FullABIParams {
                        is_constructor: false,
                        is_system_call: true,
                        r3_value: None,
                        r4_value: None,
                        r5_value: None,
                    },
                );
            (vm_launch_option, context_u128_value)
        };

        let mut calldata = Vec::with_capacity(
            constructor_calldata.len() + era_compiler_common::BYTE_LENGTH_FIELD * 4 + 4,
        );
        calldata.extend(Self::CREATE_METHOD_SELECTOR.to_be_bytes().to_vec());
        calldata.extend([0u8; 2 * era_compiler_common::BYTE_LENGTH_FIELD]);
        bytecode_hash.to_big_endian(&mut calldata[era_compiler_common::BYTE_LENGTH_FIELD + 4..]);
        calldata.extend(web3::types::H256::from_low_u64_be(96).as_bytes());
        calldata.extend(
            web3::types::H256::from_low_u64_be(constructor_calldata.len() as u64).as_bytes(),
        );
        calldata.extend(constructor_calldata);

        vm.execute::<M>(
            test_name,
            entry_address,
            caller,
            Some(context_u128_value),
            calldata,
            Some(vm_launch_option),
        )
    }
}
