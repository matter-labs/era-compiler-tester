//!
//! Runs the next-generation EraVM.
//!
//! Its interface is simpler than the old one's but different, so a compatibility layer is needed.
//!

use std::collections::HashMap;

use crate::vm::execution_result::ExecutionResult;
use anyhow::anyhow;
use lambda_vm;
use lambda_vm::state::VMState;
use lambda_vm::store::initial_decommit;
use lambda_vm::store::InMemory;
use lambda_vm::value::TaggedValue;
use lambda_vm::ExecutionOutput;
use lambda_vm::store::Storage;
use web3::types::H160;
use zkevm_assembly::Assembly;
use zkevm_opcode_defs::ethereum_types::{H256, U256};
use zkevm_tester::runners::compiler_tests::FullABIParams;
use zkevm_tester::runners::compiler_tests::StorageKey;
use zkevm_tester::runners::compiler_tests::VmExecutionContext;
use zkevm_tester::runners::compiler_tests::VmLaunchOption;

use crate::test::case::input::{
    output::{event::Event, Output},
    value::Value,
};

pub fn address_into_u256(address: H160) -> U256 {
    let mut buffer = [0; 32];
    buffer[12..].copy_from_slice(address.as_bytes());
    U256::from_big_endian(&buffer)
}

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

    let mut storage_changes = HashMap::new();
    let mut deployed_contracts = HashMap::new();

    let mut lambda_storage: HashMap<lambda_vm::store::StorageKey, U256> = HashMap::new();
    for (key, value) in storage {
        let value_bits = value.as_bytes();
        let value_u256 = U256::from_big_endian(&value_bits);
        let lambda_storage_key = lambda_vm::store::StorageKey::new(key.address, key.key);
        lambda_storage.insert(lambda_storage_key, value_u256);
    }

    for (_, contract) in contracts {
        let bytecode = contract.clone().compile_to_bytecode()?;
        let hash = zkevm_assembly::zkevm_opcode_defs::bytecode_to_code_hash(&bytecode)
            .map_err(|()| anyhow!("Failed to hash bytecode"))?;
        known_contracts.insert(U256::from_big_endian(&hash), contract);
    }

    let mut lambda_contract_storage: HashMap<U256, Vec<U256>> = HashMap::new();
    for (key, value) in known_contracts.clone() {
        let bytecode = value.clone().compile_to_bytecode()?;
        let bytecode_u256 = bytecode
            .iter()
            .map(|raw_opcode| U256::from_big_endian(raw_opcode))
            .collect();

        lambda_contract_storage.insert(key, bytecode_u256);
    }

    let mut storage = InMemory::new(lambda_contract_storage, lambda_storage);

    let initial_storage = storage.clone();

    let initial_program = initial_decommit(&mut storage, entry_address);

    let context_val = context.unwrap();

    let mut vm = VMState::new(
        initial_program,
        calldata.to_vec(),
        entry_address,
        context_val.msg_sender,
        context_val.u128_value,
    );

    if abi_params.is_constructor {
        let r1_with_constructor_bit = vm.get_register(1).value | 1.into();
        vm.set_register(2, TaggedValue::new_raw_integer(r1_with_constructor_bit));
    }
    if abi_params.is_system_call {
        let r1_with_system_bit = vm.get_register(1).value | 2.into();
        vm.set_register(2, TaggedValue::new_raw_integer(r1_with_system_bit));
    }
    vm.set_register(
        3,
        TaggedValue::new_raw_integer(abi_params.r3_value.unwrap_or_default()),
    );
    vm.set_register(
        4,
        TaggedValue::new_raw_integer(abi_params.r4_value.unwrap_or_default()),
    );
    vm.set_register(
        5,
        TaggedValue::new_raw_integer(abi_params.r5_value.unwrap_or_default()),
    );

    let (result, final_vm) = lambda_vm::run_program_with_custom_bytecode(vm, &mut storage);
    let events = merge_events(&final_vm.events);
    let output = match result {
        ExecutionOutput::Ok(output) => Output {
            return_data: chunk_return_data(&output),
            exception: false,
            events,
        },
        ExecutionOutput::Revert(output) => Output {
            return_data: chunk_return_data(&output),
            exception: true,
            events: vec![],
        },
        ExecutionOutput::Panic => Output {
            return_data: vec![],
            exception: true,
            events: vec![],
        },
    };

    for (key, value) in storage.state_storage.into_iter() {
        if initial_storage.storage_read(key).unwrap() != Some(value) {
            let mut bytes: [u8; 32] = [0;32];
            value.to_big_endian(&mut bytes);
            storage_changes.insert(StorageKey{address: key.address, key: key.key}, H256::from(bytes));
        }
    }

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

fn merge_events(events: &[lambda_vm::state::Event]) -> Vec<Event> {
    struct TmpEvent {
        topics: Vec<U256>,
        data: Vec<u8>,
        shard_id: u8,
        tx_number: u32,
    }
    let mut result = vec![];
    let mut current: Option<(usize, u32, TmpEvent)> = None;

    for message in events.into_iter() {
        let lambda_vm::state::Event {
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
