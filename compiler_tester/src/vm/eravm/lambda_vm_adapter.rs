//!
//! Runs the next-generation EraVM.
//!
//! Its interface is simpler than the old one's but different, so a compatibility layer is needed.
//!

use std::collections::HashMap;

use crate::vm::execution_result::ExecutionResult;
use anyhow::anyhow;
use lambda_vm;
use web3::contract::tokens::Tokenize;
use web3::types::H160;
use zkevm_assembly::Assembly;
use zkevm_opcode_defs::ethereum_types::{BigEndianHash, H256, U256};
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
    let mut initial_program: Vec<u8> = Vec::new();
    let address_to_find = address_into_u256(entry_address); // This is not the correct address, it should find the address from deployer system contract
    let bytecode = known_contracts.get(&address_to_find).map(|assembly| assembly.clone().compile_to_bytecode().unwrap()).unwrap();
    for byte in bytecode {
        for b in byte {
            initial_program.push(b);
        }
    }

    let mut storage_changes = HashMap::new();
    let mut deployed_contracts = HashMap::new();

    let result = lambda_vm::run_program_with_custom_bytecode(initial_program);

    let mut result_vec: [u8; 32] = [0; 32];
    result.0.to_big_endian(&mut result_vec);
    let output = Output {
        return_data: chunk_return_data(&result_vec),
        exception: false,
        events: vec![],
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
