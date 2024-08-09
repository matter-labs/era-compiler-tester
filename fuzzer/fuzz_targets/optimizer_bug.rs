//!
//! This module contains the fuzzing target for the simple contract.
//!

#![no_main]

use libfuzzer_sys::fuzz_target;

mod common;

fuzz_target!(|data: u8| {
    // Fuzzing case definition
    let case = common::FuzzingCase {
        contract_path: String::from("fuzzer/fuzz_contracts/optimizer_bug/optimizer_bug.sol"),
        function_name: String::from("function_to_fuzz"),
        input_types: vec![
            common::TypeVariant::integer_unsigned(8),
            common::TypeVariant::boolean(),
        ],
        inputs: vec![common::integer_literal(data), common::boolean_literal(true)],
        expected_output: common::integer_literal(1),
    };

    // Generate fuzzing test
    let test = common::gen_fuzzing_test(case).expect("Error: cannot build fuzzing test!");

    // Run test and check the results
    let result = common::build_and_run(test).expect("Error: cannot execute fuzzing test!");

    // Check if the test was successful
    assert!(result.is_successful())
});
