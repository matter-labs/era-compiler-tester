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
use lambda_vm::store;
use lambda_vm::store::initial_decommit;
use lambda_vm::store::InMemory;
use lambda_vm::value::TaggedValue;
use web3::contract::tokens::Tokenize;
use web3::types::H160;
use zkevm_assembly::Assembly;
use zkevm_opcode_defs::ethereum_types::{BigEndianHash, H256, U256};
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

    let initial_program = initial_decommit(&mut storage, entry_address);

    let mut storage = InMemory::new_empty();

    let mut vm = VMState::new(initial_program, calldata.to_vec(), entry_address, context.unwrap().msg_sender);

    if abi_params.is_constructor {
        vm.registers[1] |= TaggedValue::new_raw_integer(1.into());
    }
    if abi_params.is_system_call {
        vm.registers[1] |= TaggedValue::new_raw_integer(2.into());
    }
    vm.registers[3] = TaggedValue::new_raw_integer(abi_params.r3_value.unwrap_or_default());
    vm.registers[4] = TaggedValue::new_raw_integer(abi_params.r4_value.unwrap_or_default());
    vm.registers[5] = TaggedValue::new_raw_integer(abi_params.r5_value.unwrap_or_default());

    let result = lambda_vm::run_program_with_custom_bytecode(vm, &mut storage);

    let mut result_vec: [u8; 32] = [0; 32];
    result.0.to_big_endian(&mut result_vec);
    let output = Output {
        return_data: chunk_return_data(&result_vec),
        exception: false,
        events: vec![],
    };

    dbg!(output.clone());
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
