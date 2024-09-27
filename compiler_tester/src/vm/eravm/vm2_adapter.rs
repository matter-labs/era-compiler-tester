//!
//! Runs the next-generation EraVM.
//!
//! Its interface is simpler than the old one's but different, so a compatibility layer is needed.
//!

use std::collections::HashMap;

use crate::vm::execution_result::ExecutionResult;
use anyhow::anyhow;
use vm2::initial_decommit;
use vm2::ExecutionEnd;
use vm2::Program;
use vm2::World;
use zkevm_assembly::Assembly;
use zkevm_opcode_defs::ethereum_types::{BigEndianHash, H256, U256};
use zkevm_tester::compiler_tests::FullABIParams;
use zkevm_tester::compiler_tests::StorageKey;
use zkevm_tester::compiler_tests::VmExecutionContext;
use zkevm_tester::compiler_tests::VmLaunchOption;

use crate::test::case::input::{
    output::{event::Event, Output},
    value::Value,
};

pub fn run_vm(
    contracts: HashMap<web3::ethabi::Address, Assembly>,
    calldata: &[u8],
    storage: HashMap<StorageKey, H256>,
    entry_address: web3::ethabi::Address,
    context: Option<VmExecutionContext>,
    vm_launch_option: VmLaunchOption,
    mut known_contracts: HashMap<U256, Assembly>,
    default_aa_code_hash: U256,
    evm_interpreter_code_hash: U256,
) -> anyhow::Result<(
    ExecutionResult,
    HashMap<StorageKey, H256>,
    HashMap<web3::ethabi::Address, Assembly>,
)> {
    let abi_params = match vm_launch_option {
        VmLaunchOption::Call => FullABIParams {
            is_constructor: false,
            is_system_call: false,
            r3_value: None,
            r4_value: None,
            r5_value: None,
        },
        VmLaunchOption::Constructor => FullABIParams {
            is_constructor: true,
            is_system_call: false,
            r3_value: None,
            r4_value: None,
            r5_value: None,
        },
        VmLaunchOption::ManualCallABI(abiparams) => abiparams,
        x => return Err(anyhow!("Unsupported launch option {x:?}")),
    };

    for (_, contract) in contracts {
        let bytecode = contract.clone().compile_to_bytecode()?;
        let hash = zkevm_assembly::zkevm_opcode_defs::bytecode_to_code_hash(&bytecode)
            .map_err(|()| anyhow!("Failed to hash bytecode"))?;
        known_contracts.insert(U256::from_big_endian(&hash), contract);
    }

    let context = context.unwrap_or_default();

    let mut world = TestWorld {
        storage,
        contracts: known_contracts.clone(),
    };
    let initial_program = initial_decommit(&mut world, entry_address);

    let mut vm = vm2::VirtualMachine::new(
        entry_address,
        initial_program,
        context.msg_sender,
        calldata.to_vec(),
        // zkevm_tester subtracts this constant, I don't know why
        u32::MAX - 0x80000000,
        vm2::Settings {
            default_aa_code_hash: default_aa_code_hash.into(),
            evm_interpreter_code_hash: evm_interpreter_code_hash.into(),
            hook_address: 0,
        },
    );

    if abi_params.is_constructor {
        vm.state.registers[2] |= 1.into();
    }
    if abi_params.is_system_call {
        vm.state.registers[2] |= 2.into();
    }
    vm.state.registers[3] = abi_params.r3_value.unwrap_or_default();
    vm.state.registers[4] = abi_params.r4_value.unwrap_or_default();
    vm.state.registers[5] = abi_params.r5_value.unwrap_or_default();

    let mut storage_changes = HashMap::new();
    let mut deployed_contracts = HashMap::new();

    let output = match vm.run(&mut world) {
        ExecutionEnd::ProgramFinished(return_value) => {
            // Only successful transactions can have side effects
            // The VM doesn't undo side effects done in the initial frame
            // because that would mess with the bootloader.

            storage_changes = vm
                .world_diff
                .get_storage_state()
                .iter()
                .map(|(&(address, key), (_, value))| {
                    (StorageKey { address, key }, H256::from_uint(value))
                })
                .collect::<HashMap<_, _>>();
            deployed_contracts = vm
                .world_diff
                .get_storage_state()
                .iter()
                .filter_map(|((address, key), (_, value))| {
                    if *address == *zkevm_assembly::zkevm_opcode_defs::system_params::DEPLOYER_SYSTEM_CONTRACT_ADDRESS {
                        let mut buffer = [0u8; 32];
                        key.to_big_endian(&mut buffer);
                        let deployed_address = web3::ethabi::Address::from_slice(&buffer[12..]);
                        if let Some(code) = known_contracts.get(&value) {
                            Some((deployed_address, code.clone()))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                })
                .collect::<HashMap<_, _>>();

            Output {
                return_data: chunk_return_data(&return_value),
                exception: false,
                events: merge_events(vm.world_diff.events()),
            }
        }
        ExecutionEnd::Reverted(return_value) => Output {
            return_data: chunk_return_data(&return_value),
            exception: true,
            events: vec![],
        },
        ExecutionEnd::Panicked => Output {
            return_data: vec![],
            exception: true,
            events: vec![],
        },
        ExecutionEnd::SuspendedOnHook { .. } => unreachable!(),
    };

    Ok((
        ExecutionResult {
            output,
            cycles: 0,
            ergs: 0,
            gas: 0,
        },
        storage_changes,
        deployed_contracts,
    ))
}

struct TestWorld {
    storage: HashMap<StorageKey, H256>,
    contracts: HashMap<U256, Assembly>,
}

impl World for TestWorld {
    fn decommit(&mut self, hash: U256) -> Program {
        let Some(bytecode) = self
            .contracts
            .get(&hash)
            .map(|assembly| assembly.clone().compile_to_bytecode().unwrap())
        else {
            // This case only happens when a contract fails to deploy but the assumes it did deploy
            return Program::new(vec![vm2::Instruction::from_invalid()], vec![]);
        };
        let instructions = bytecode
            .iter()
            .flat_map(|x| {
                x.chunks(8)
                    .map(|x| u64::from_be_bytes(x.try_into().unwrap()))
            })
            .collect::<Vec<_>>();

        Program::new(
            vm2::decode::decode_program(&instructions, false),
            bytecode
                .iter()
                .map(|x| U256::from_big_endian(x))
                .collect::<Vec<_>>(),
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

    fn cost_of_writing_storage(
        &mut self,
        contract: web3::types::H160,
        key: U256,
        new_value: U256,
    ) -> u32 {
        0
    }

    fn is_free_storage_slot(&self, contract: &web3::types::H160, key: &U256) -> bool {
        self.storage.contains_key(&StorageKey {
            address: *contract,
            key: *key,
        })
    }
}

fn chunk_return_data(bytes: &[u8]) -> Vec<Value> {
    let iter = bytes.chunks_exact(32);
    let remainder = iter.remainder();
    let mut res = iter
        .map(U256::from_big_endian)
        .map(Value::Certain)
        .collect::<Vec<_>>();
    if !remainder.is_empty() {
        let mut last = [0; 32];
        last[..remainder.len()].copy_from_slice(remainder);
        res.push(Value::Certain(U256::from_big_endian(&last)));
    }
    res
}

fn merge_events(events: &[vm2::Event]) -> Vec<Event> {
    struct TmpEvent {
        topics: Vec<U256>,
        data: Vec<u8>,
        shard_id: u8,
        tx_number: u32,
    }
    let mut result = vec![];
    let mut current: Option<(usize, u32, TmpEvent)> = None;

    for message in events.into_iter() {
        let vm2::Event {
            shard_id,
            is_first,
            tx_number,
            key,
            value,
        } = *message;
        let tx_number = tx_number.into();

        if !is_first {
            if let Some((mut remaining_data_length, mut remaining_topics, mut event)) =
                current.take()
            {
                if event.shard_id != shard_id || event.tx_number != tx_number {
                    continue;
                }

                for el in [key, value].iter() {
                    if remaining_topics != 0 {
                        event.topics.push(*el);
                        remaining_topics -= 1;
                    } else if remaining_data_length != 0 {
                        let mut bytes = [0; 32];
                        el.to_big_endian(&mut bytes);
                        if remaining_data_length >= 32 {
                            event.data.extend_from_slice(&bytes);
                            remaining_data_length -= 32;
                        } else {
                            event
                                .data
                                .extend_from_slice(&bytes[..remaining_data_length]);
                            remaining_data_length = 0;
                        }
                    }
                }

                if remaining_data_length != 0 || remaining_topics != 0 {
                    current = Some((remaining_data_length, remaining_topics, event))
                } else {
                    result.push(event);
                }
            }
        } else {
            // start new one. First take the old one only if it's well formed
            if let Some((remaining_data_length, remaining_topics, event)) = current.take() {
                if remaining_data_length == 0 && remaining_topics == 0 {
                    result.push(event);
                }
            }

            // split key as our internal marker. Ignore higher bits
            let mut num_topics = key.0[0] as u32;
            let mut data_length = (key.0[0] >> 32) as usize;
            let mut buffer = [0u8; 32];
            value.to_big_endian(&mut buffer);

            let (topics, data) = if num_topics == 0 && data_length == 0 {
                (vec![], vec![])
            } else if num_topics == 0 {
                data_length -= 32;
                (vec![], buffer.to_vec())
            } else {
                num_topics -= 1;
                (vec![value], vec![])
            };

            let new_event = TmpEvent {
                shard_id,
                tx_number,
                topics,
                data,
            };

            current = Some((data_length, num_topics, new_event))
        }
    }

    // add the last one
    if let Some((remaining_data_length, remaining_topics, event)) = current.take() {
        if remaining_data_length == 0 && remaining_topics == 0 {
            result.push(event);
        }
    }

    result
        .iter()
        .filter_map(|event| {
            let mut address_bytes = [0; 32];
            event.topics[0].to_big_endian(&mut address_bytes);
            let address = web3::ethabi::Address::from_slice(&address_bytes[12..]);

            // Filter out events that are from system contracts
            if address.as_bytes().iter().rev().skip(2).all(|x| *x == 0) {
                return None;
            }
            let topics = event.topics[1..]
                .iter()
                .cloned()
                .map(Value::Certain)
                .collect();
            let values = chunk_return_data(&event.data);
            Some(Event::new(Some(address), topics, values))
        })
        .collect()
}
