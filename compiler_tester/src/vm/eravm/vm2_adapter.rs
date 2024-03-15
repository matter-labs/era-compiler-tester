use std::collections::HashMap;
use vm2::World;
use web3::ethabi::Address;
use zkevm_assembly::{zkevm_opcode_defs::bytecode_to_code_hash, Assembly};
use zkevm_opcode_defs::ethereum_types::{BigEndianHash, H256, U256};
use zkevm_tester::runners::compiler_tests::{
    FullABIParams, StorageKey, VmExecutionContext, VmLaunchOption,
};

use crate::test::case::input::output::Output;

use super::execution_result::ExecutionResult;

pub fn run_vm(
    contracts: HashMap<Address, Assembly>,
    calldata: &[u8],
    storage: HashMap<StorageKey, H256>,
    entry_address: Address,
    context: Option<VmExecutionContext>,
    vm_launch_option: VmLaunchOption,
    mut known_contracts: HashMap<U256, Assembly>,
    default_aa_code_hash: U256,
) -> anyhow::Result<ExecutionResult> {
    let abi_params = match vm_launch_option {
        VmLaunchOption::Constructor => FullABIParams {
            is_constructor: true,
            is_system_call: false,
            r3_value: None,
            r4_value: None,
            r5_value: None,
        },
        VmLaunchOption::ManualCallABI(abiparams) => abiparams,
        _ => return Err(anyhow::anyhow!("Unsupported launch option")),
    };

    for (_, contract) in contracts {
        let bytecode = contract.clone().compile_to_bytecode().unwrap();
        let hash = bytecode_to_code_hash(&bytecode).unwrap();
        known_contracts.insert(U256::from_big_endian(&hash), contract);
    }

    let mut vm = vm2::State::new(
        Box::new(TestWorld {
            storage,
            contracts: known_contracts,
        }),
        entry_address,
        calldata.to_vec(),
    );

    let output = match vm.run() {
        Ok(_) => Output {
            return_data: vec![],
            exception: false,
            events: vec![],
        },
        Err(e) => {
            dbg!(e, vm.current_frame.gas);
            Output {
                return_data: vec![],
                exception: true,
                events: vec![],
            }
        }
    };

    Ok(ExecutionResult {
        output,
        cycles: 0,
        ergs: 0,
    })
}

struct TestWorld {
    storage: HashMap<StorageKey, H256>,
    contracts: HashMap<U256, Assembly>,
}
impl World for TestWorld {
    fn decommit(
        &mut self,
        hash: U256,
    ) -> (std::sync::Arc<[vm2::Instruction]>, std::sync::Arc<[U256]>) {
        let bytecode = self
            .contracts
            .get(&hash)
            .unwrap()
            .clone()
            .compile_to_bytecode()
            .unwrap();
        let instructions = bytecode
            .iter()
            .flat_map(|x| {
                x.chunks(8)
                    .map(|x| u64::from_be_bytes(x.try_into().unwrap()))
            })
            .collect::<Vec<_>>();

        (
            vm2::decode::decode_program(&instructions).into(),
            bytecode
                .iter()
                .map(|x| U256::from_big_endian(x))
                .collect::<Vec<_>>()
                .into(),
        )
    }

    fn read_storage(
        &mut self,
        contract: zkevm_opcode_defs::ethereum_types::H160,
        key: U256,
    ) -> U256 {
        self.storage
            .get(&StorageKey {
                address: contract,
                key,
            })
            .map(|h| h.into_uint())
            .unwrap_or(U256::zero())
    }
}
