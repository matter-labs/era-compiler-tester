//!
//! Runs the in-circuits VM.
//!
//! Only base layer circuits will be generated and validated(without proving).
//!

use std::collections::HashMap;

///
/// Run out-of-circuits and in-circuits VMs with a special bootloader to set context values.
/// Returns storage after execution.
///
pub fn run_vm(
    mut bootloader: zkevm_assembly::Assembly,
    calldata: &[u8],
    storage: HashMap<zkevm_tester::runners::compiler_tests::StorageKey, web3::types::H256>,
    context: zkevm_tester::runners::compiler_tests::VmExecutionContext,
    vm_launch_option: zkevm_tester::runners::compiler_tests::VmLaunchOption,
    known_contracts: HashMap<web3::types::U256, zkevm_assembly::Assembly>,
    default_aa_code_hash: web3::types::U256,
    evm_interpreter_code_hash: web3::types::U256,
) -> HashMap<zkevm_tester::runners::compiler_tests::StorageKey, web3::types::H256> {
    let entry_point_code = bootloader.compile_to_bytecode().unwrap();
    let known_contracts = known_contracts.into_iter().map(|(hash, mut asm)| (hash, asm.compile_to_bytecode().unwrap())).collect();
    let mut storage_input: HashMap<web3::types::Address, HashMap<web3::types::U256, web3::types::U256>> = HashMap::new();
    for (key, value) in storage.into_iter() {
        storage_input.entry(key.address).or_default().insert(key.key, crate::utils::h256_to_u256(&value));
    }
    let initial_heap_content = encode_bootloader_memory(context.this_address, context.msg_sender, context.u128_value, calldata, vm_launch_option);

    let new_storage = zkevm_test_harness::compiler_tests_runner::compiler_tests_run(
        entry_point_code,
        default_aa_code_hash,
        evm_interpreter_code_hash,
        known_contracts,
        storage_input,
        initial_heap_content,
        <usize>::MAX
    );
    let mut result_storage = HashMap::new();
    for(address, inner) in new_storage.into_iter() {
        for(key, value) in inner.into_iter() {
            result_storage.insert(
                zkevm_tester::runners::compiler_tests::StorageKey {
                    address,
                    key
                }, crate::utils::u256_to_h256(&value)
            );
        }
    }
    result_storage
}

// 0-32: `to` address, lower 20 bytes.
// 32-64: `from` address, lower 20 bytes.
// 64-96: `is_constructor` flag.
// 96-128: `is_system` flag.
// 128-160: `extra_abi_data_1` the first extra abi param.
// 160-192: `extra_abi_data_2` the second extra abi param.
// 192-224: `extra_abi_data_3` the third extra abi param.
// 224-256: `context_u128_value` for the call, lower 16 bytes.
// 256-288: calldata length.
// 288-`288+calldata length`: calldata.
fn encode_bootloader_memory(
    entry_address: web3::types::Address,
    caller: web3::types::Address,
    u128_value: u128,
    calldata: &[u8],
    vm_launch_option: zkevm_tester::runners::compiler_tests::VmLaunchOption,
) -> Vec<u8> {
    let mut data = vec![0u8; 12];
    data.extend(entry_address.as_bytes());

    data.extend(vec![0u8; 12]);
    data.extend(caller.as_bytes());

    let mut is_constructor = web3::types::U256::zero();
    let mut is_system = web3::types::U256::zero();
    let mut extra_abi_data_1 = web3::types::U256::zero();
    let mut extra_abi_data_2 = web3::types::U256::zero();
    let mut extra_abi_data_3 = web3::types::U256::zero();
    match vm_launch_option {
        zkevm_tester::runners::compiler_tests::VmLaunchOption::ManualCallABI(params) => {
            if params.is_constructor {
                is_constructor = web3::types::U256::one();
            }
            if params.is_system_call {
                is_system = web3::types::U256::one();
            }
            extra_abi_data_1 = params.r3_value.unwrap_or_default();
            extra_abi_data_2 = params.r4_value.unwrap_or_default();
            extra_abi_data_3 = params.r5_value.unwrap_or_default();
        },
        zkevm_tester::runners::compiler_tests::VmLaunchOption::Constructor => {
            is_constructor = web3::types::U256::one();
        },
        zkevm_tester::runners::compiler_tests::VmLaunchOption::Call => {},
        _ => panic!("Unsupported launch option")
    };

    data.extend(vec![0u8; 32 * 5]);
    is_constructor.to_big_endian(&mut data[64..96]);
    is_system.to_big_endian(&mut data[96..128]);
    extra_abi_data_1.to_big_endian(&mut data[128..160]);
    extra_abi_data_2.to_big_endian(&mut data[160..192]);
    extra_abi_data_3.to_big_endian(&mut data[192..224]);

    data.extend(vec![0u8; 32 * 2]);
    let context_u128 = web3::types::U256::from(u128_value);
    let calldata_len = web3::types::U256::from(calldata.len());
    context_u128.to_big_endian(&mut data[224..256]);
    calldata_len.to_big_endian(&mut data[256..288]);

    data.extend(calldata);
    data
}