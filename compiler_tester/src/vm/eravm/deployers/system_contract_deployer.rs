//!
//! The EraVM system contract deployer implementation.
//!

use crate::vm::eravm::deployers::EraVMDeployer;
use crate::vm::eravm::EraVM;
use crate::vm::execution_result::ExecutionResult;

///
/// The EraVM system contract deployer implementation.
///
#[derive(Debug, Clone)]
pub struct SystemContractDeployer;

impl SystemContractDeployer {
    /// The create method selector.
    const ERAVM_CREATE_METHOD_SIGNATURE: &'static str = "create(bytes32,bytes32,bytes)";

    /// The create method selector.
    const EVM_CREATE_METHOD_SIGNATURE: &'static str = "createEVM(bytes)";
}

impl EraVMDeployer for SystemContractDeployer {
    fn new() -> Self {
        Self
    }

    fn deploy_eravm<const M: bool>(
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

            let vm_launch_option = zkevm_tester::compiler_tests::VmLaunchOption::ManualCallABI(
                zkevm_tester::compiler_tests::FullABIParams {
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
            let vm_launch_option = zkevm_tester::compiler_tests::VmLaunchOption::ManualCallABI(
                zkevm_tester::compiler_tests::FullABIParams {
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
        calldata.extend(crate::utils::selector(Self::ERAVM_CREATE_METHOD_SIGNATURE));
        calldata.extend([0u8; 2 * era_compiler_common::BYTE_LENGTH_FIELD]);
        bytecode_hash.to_big_endian(&mut calldata[era_compiler_common::BYTE_LENGTH_FIELD + 4..]);
        calldata.extend(
            web3::types::H256::from_low_u64_be((3 * era_compiler_common::BYTE_LENGTH_FIELD) as u64)
                .as_bytes(),
        );
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

    fn deploy_evm<const M: bool>(
        &mut self,
        test_name: String,
        caller: web3::types::Address,
        init_code: Vec<u8>,
        constructor_calldata: Vec<u8>,
        value: Option<u128>,
        vm: &mut EraVM,
    ) -> anyhow::Result<ExecutionResult> {
        let context_u128_value;
        let vm_launch_option;
        let mut entry_address = web3::types::Address::from_low_u64_be(
            zkevm_opcode_defs::ADDRESS_CONTRACT_DEPLOYER.into(),
        );

        if M {
            context_u128_value = 0;

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

            vm_launch_option = zkevm_tester::compiler_tests::VmLaunchOption::ManualCallABI(
                zkevm_tester::compiler_tests::FullABIParams {
                    is_constructor: false,
                    is_system_call: true,
                    r3_value: r3,
                    r4_value: r4,
                    r5_value: r5,
                },
            );
        } else {
            if let Some(value) = value {
                context_u128_value = value;
                vm.mint_ether(
                    web3::types::Address::from_low_u64_be(
                        zkevm_opcode_defs::ADDRESS_CONTRACT_DEPLOYER.into(),
                    ),
                    web3::types::U256::from(value),
                );
            } else {
                context_u128_value = 0;
            }

            vm_launch_option = zkevm_tester::compiler_tests::VmLaunchOption::ManualCallABI(
                zkevm_tester::compiler_tests::FullABIParams {
                    is_constructor: false,
                    is_system_call: true,
                    r3_value: None,
                    r4_value: None,
                    r5_value: None,
                },
            );
        }

        let mut calldata = Vec::with_capacity(
            era_compiler_common::BYTE_LENGTH_X32
                + era_compiler_common::BYTE_LENGTH_FIELD * 2
                + init_code.len()
                + constructor_calldata.len(),
        );
        calldata.extend(crate::utils::selector(Self::EVM_CREATE_METHOD_SIGNATURE));
        calldata.extend(
            web3::types::H256::from_low_u64_be(era_compiler_common::BYTE_LENGTH_FIELD as u64)
                .as_bytes(),
        );
        calldata.extend(
            web3::types::H256::from_low_u64_be(
                (init_code.len() + constructor_calldata.len()) as u64,
            )
            .as_bytes(),
        );
        calldata.extend(init_code);
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
