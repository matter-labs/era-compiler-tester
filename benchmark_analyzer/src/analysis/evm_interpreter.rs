//!
//! Collects definitions related to the analysis of EVM interpreter tests.
//!

use std::collections::BTreeMap;

use crate::model::benchmark::test::toolchain::codegen::versioned::executable::run::Run;
use crate::model::evm_interpreter::OPCODES;
use crate::results::group::Group;
use crate::results::run_description::RunDescription;

const OPTIMIZE_FOR_CYCLES: &str = "+M3B3";

///
/// Returns `true` if the group collects measurements of EVM Interpreter tests
/// compiled for maximum performance.
///
pub fn is_evm_interpreter_cycles_tests_group(group: &Group<'_>) -> bool {
    matches!(
        group,
        Group::EVMInterpreter {
            optimizations: OPTIMIZE_FOR_CYCLES,
            ..
        }
    )
}

///
/// Returns the EVM interpreter ergs/gas ratio for every EVM bytecode.
///
pub fn opcode_cost_ratios<'a>(
    group: &BTreeMap<&'a str, (RunDescription<'a>, &'a Run)>,
) -> Vec<(String, f64)> {
    let mut results = Vec::new();

    for evm_opcode in OPCODES.into_iter() {
        // Case name corresponds to the EVM bytecode
        // Collect three last #fallback's to get the gas and ergs measurements
        let runs = group
            .values()
            .filter_map(
                |(description, run)| match &description.test_metadata.selector.case {
                    Some(case) if case == evm_opcode => {
                        match &description.test_metadata.selector.input {
                            Some(input) if input.is_fallback() => Some(*run),
                            _ => None,
                        }
                    }
                    _ => None,
                },
            )
            .collect::<Vec<&'a Run>>();
        let [_skip, full, template]: [&'a Run; 3] = runs
            .try_into()
            .unwrap_or_else(|_| panic!("Failed to extract three #fallback tests from the EVM interpreter tests attributed to the opcode {evm_opcode}"));

        let ergs_difference = full.ergs as i64 - template.ergs as i64;
        let gas_difference = full.gas as i64 - template.gas as i64;
        let ergs_gas_ratio = (ergs_difference as f64) / (gas_difference as f64);
        results.push((evm_opcode.to_owned(), ergs_gas_ratio));
    }
    results
}
