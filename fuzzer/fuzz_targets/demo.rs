//!
//! The fuzzer demo.
//!

#![no_main]

/// This module contains the fuzzing target for the simple contract.
use libfuzzer_sys::fuzz_target;

pub(crate) mod common;

fuzz_target!(|data: u8| {
    // Fuzzing case definition
    let case = common::FuzzingCase {
        contract_path: String::from("fuzz/fuzz_contracts/demo/demo.sol"),
        function_name: String::from("should_always_return_0"),
        input_types: vec![common::TypeVariant::integer_unsigned(8)],
        inputs: vec![common::integer_literal(data)],
        expected_output: common::integer_literal(0),
    };

    // Generate fuzzing test
    println!("Generating fuzzing test with input {data}...");
    let test = common::gen_fuzzing_test(case).expect("Error: cannot build fuzzing test!");

    // Run test and check the results
    let result = common::build_and_run(test).expect("Error: cannot execute fuzzing test!");

    // Check if the test was successful
    assert!(result.is_successful())
});
