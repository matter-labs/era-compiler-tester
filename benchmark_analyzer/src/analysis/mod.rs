//!
//! Provides tools for collecting and comparing benchmark results.
//!

pub mod evm_interpreter;

use std::collections::BTreeMap;

use evm_interpreter::is_evm_interpreter_cycles_tests_group;
use evm_interpreter::opcode_cost_ratios;

use crate::model::benchmark::test::toolchain::codegen::versioned::executable::run::Run;
use crate::model::benchmark::Benchmark;
use crate::results::group::Group;
use crate::results::run_description::RunDescription;
use crate::results::Results;
use crate::util::btreemap::cross_join_filter_map;
use crate::util::btreemap::intersect_keys;
use crate::util::btreemap::intersect_map;

type GroupRuns<'a> = BTreeMap<&'a str, (RunDescription<'a>, &'a Run)>;

///
/// Collects measurements from a benchmark into groups.
/// Groups may intersect.
///
fn collect_runs(benchmark: &Benchmark) -> BTreeMap<Group<'_>, GroupRuns> {
    let mut result: BTreeMap<Group<'_>, GroupRuns> = BTreeMap::new();
    for (test_identifier, test) in benchmark.tests.iter() {
        for (_toolchain, toolchain_group) in test.toolchain_groups.iter() {
            for (codegen, codegen_group) in toolchain_group.codegen_groups.iter() {
                for (version, versioned_group) in codegen_group.versioned_groups.iter() {
                    for (mode, executable) in versioned_group.executables.iter() {
                        for tag in test
                            .metadata
                            .tags
                            .iter()
                            .map(Some)
                            .chain(std::iter::once(None))
                        {
                            let tag = tag.map(|tag| tag.as_str());
                            if codegen == "I"
                                || (tag == Some("EVMInterpreter") && codegen != "Y")
                                || (tag != Some("Precompiles") && codegen == "NoCodegen")
                            {
                                continue;
                            }

                            let run_description = RunDescription {
                                test_metadata: &test.metadata,
                                version,
                                codegen,
                                mode,
                                executable_metadata: &executable.metadata,
                                run: &executable.run,
                            };
                            result
                                .entry(Group::from_tag(tag, Some(codegen), Some(mode)))
                                .or_default()
                                .insert(
                                    test_identifier.as_str(),
                                    (run_description, &executable.run),
                                );
                        }
                    }
                }
            }
        }
    }
    result
}

///
/// Compare two benchmark runs [reference] and [candidate] by groups.
/// Every resulting group is either:
/// - a result of comparison of a group from [reference] with a group from [candidate] sharing the same name
/// - or a result of comparing two distinct groups from [reference] and
///   [candidate] for which [custom_group_comparisons] returned `true`.
///
pub fn compare<'a>(
    reference: &'a Benchmark,
    candidate: &'a Benchmark,
    custom_group_comparisons: impl Fn(&Group, &Group) -> bool,
) -> Vec<(Group<'a>, Results<'a>)> {
    let groups = {
        let reference_runs: BTreeMap<Group<'a>, GroupRuns<'a>> = collect_runs(reference);
        let candidate_runs: BTreeMap<Group<'a>, GroupRuns<'a>> = collect_runs(candidate);

        let comparisons: Vec<(Group<'a>, GroupRuns<'a>, GroupRuns<'a>)> =
            cross_join_filter_map(&reference_runs, &candidate_runs, |g1, g2| {
                if custom_group_comparisons(g1, g2) {
                    Some(Group::new_comparison(g1, g2))
                } else {
                    None
                }
            });

        intersect_keys(reference_runs, candidate_runs).chain(comparisons)
    };

    let results: Vec<(Group<'_>, Results<'_>)> = groups
        .into_iter()
        .map(|(group_name, reference_tests, candidate_tests)| {
            let ratios = if is_evm_interpreter_cycles_tests_group(&group_name) {
                Some((
                    opcode_cost_ratios(&reference_tests),
                    opcode_cost_ratios(&candidate_tests),
                ))
            } else {
                None
            };

            let runs: Vec<(RunDescription, &Run, &Run)> = intersect_map(
                reference_tests,
                candidate_tests,
                |_id, (metadata, run_reference), (_, run_candidate)| {
                    (metadata, run_reference, run_candidate)
                },
            )
            .collect();
            let results = {
                let mut runs = compare_runs(runs);

                if let Some((reference_ratios, candidate_ratios)) = ratios {
                    runs.set_evm_interpreter_ratios(reference_ratios, candidate_ratios);
                }
                runs
            };
            (group_name, results)
        })
        .collect();

    results
}

///
/// Compare two sets of measurements.
/// The parameter `[run]` is a vector of triples where each element contains:
/// - metadata for a test,
/// - measurement in the first set,
/// - measurement in the second set.
///
fn compare_runs<'a>(runs: Vec<(RunDescription<'a>, &'a Run, &'a Run)>) -> Results<'a> {
    let elements_number = runs.len();

    let mut size_factors = Vec::with_capacity(elements_number);
    let mut size_best = 1.0;
    let mut size_worst = 1.0;
    let mut size_negatives: Vec<(f64, RunDescription<'a>)> = Vec::with_capacity(elements_number);
    let mut size_positives: Vec<(f64, RunDescription<'a>)> = Vec::with_capacity(elements_number);
    let mut size_total_reference: u64 = 0;
    let mut size_total_candidate: u64 = 0;

    let mut cycles_factors = Vec::with_capacity(elements_number);
    let mut cycles_best = 1.0;
    let mut cycles_worst = 1.0;
    let mut cycles_negatives: Vec<(f64, RunDescription<'a>)> = Vec::with_capacity(elements_number);
    let mut cycles_positives: Vec<(f64, RunDescription<'a>)> = Vec::with_capacity(elements_number);
    let mut cycles_total_reference: u64 = 0;
    let mut cycles_total_candidate: u64 = 0;

    let mut ergs_factors = Vec::with_capacity(elements_number);
    let mut ergs_best = 1.0;
    let mut ergs_worst = 1.0;
    let mut ergs_negatives: Vec<(f64, RunDescription<'a>)> = Vec::with_capacity(elements_number);
    let mut ergs_positives: Vec<(f64, RunDescription<'a>)> = Vec::with_capacity(elements_number);
    let mut ergs_total_reference: u64 = 0;
    let mut ergs_total_candidate: u64 = 0;

    let mut gas_factors = Vec::with_capacity(elements_number);
    let mut gas_best = 1.0;
    let mut gas_worst = 1.0;
    let mut gas_negatives = Vec::with_capacity(elements_number);
    let mut gas_positives = Vec::with_capacity(elements_number);
    let mut gas_total_reference: u64 = 0;
    let mut gas_total_candidate: u64 = 0;

    for (description, reference, candidate) in runs.into_iter() {
        let file_path = &description.test_metadata.selector.project;
        // FIXME: ad-hoc patch
        if file_path.contains(crate::model::evm_interpreter::TEST_PATH) {
            if let Some(input) = &description.test_metadata.selector.input {
                if input.is_deploy() {
                    continue;
                }
            }
        }

        let cycles_reference = reference.average_cycles();
        let cycles_candidate = candidate.average_cycles();
        let cycles_factor = (cycles_candidate as f64) / (cycles_reference as f64);
        if cycles_candidate > cycles_reference {
            cycles_negatives.push((cycles_factor, description.clone()));
        }
        if cycles_candidate < cycles_reference {
            cycles_positives.push((cycles_factor, description.clone()));
        }
        if cycles_factor < cycles_best {
            cycles_best = cycles_factor;
        }
        if cycles_factor > cycles_worst {
            cycles_worst = cycles_factor;
        }
        cycles_factors.push(cycles_factor);
        cycles_total_reference += cycles_reference;
        cycles_total_candidate += cycles_candidate;

        let ergs_reference = reference.average_ergs();
        let ergs_candidate = candidate.average_ergs();
        let ergs_factor = (ergs_candidate as f64) / (ergs_reference as f64);
        if ergs_candidate > ergs_reference {
            ergs_negatives.push((ergs_factor, description.clone()));
        }
        if ergs_candidate < ergs_reference {
            ergs_positives.push((ergs_factor, description.clone()));
        }
        if ergs_factor < ergs_best {
            ergs_best = ergs_factor;
        }
        if ergs_factor > ergs_worst {
            ergs_worst = ergs_factor;
        }
        ergs_factors.push(ergs_factor);
        ergs_total_reference += ergs_reference;
        ergs_total_candidate += ergs_candidate;

        let gas_reference = reference.average_gas();
        let gas_candidate = candidate.average_gas();
        let gas_factor = (gas_candidate as f64) / (gas_reference as f64);
        if gas_candidate > gas_reference {
            gas_negatives.push((gas_factor, description.clone()));
        }
        if gas_candidate < gas_reference {
            gas_positives.push((gas_factor, description.clone()));
        }
        if gas_factor < gas_best {
            gas_best = gas_factor;
        }
        if gas_factor > gas_worst {
            gas_worst = gas_factor;
        }
        gas_factors.push(gas_factor);
        gas_total_reference += gas_reference;
        gas_total_candidate += gas_candidate;

        let reference_size = if !reference.size.is_empty() {
            reference.average_size()
        } else {
            continue;
        };
        let candidate_size = if !candidate.size.is_empty() {
            candidate.average_size()
        } else {
            continue;
        };
        let size_factor = (candidate_size as f64) / (reference_size as f64);
        if candidate_size > reference_size {
            size_negatives.push((size_factor, description.clone()));
        }
        if candidate_size < reference_size {
            size_positives.push((size_factor, description.clone()));
        }
        if size_factor < size_best {
            size_best = size_factor;
        }
        if size_factor > size_worst {
            size_worst = size_factor;
        }
        size_factors.push(size_factor);
        size_total_reference += reference_size;
        size_total_candidate += candidate_size;
    }

    let size_total = (size_total_candidate as f64) / (size_total_reference as f64);

    let cycles_total = (cycles_total_candidate as f64) / (cycles_total_reference as f64);

    let ergs_total = (ergs_total_candidate as f64) / (ergs_total_reference as f64);

    let gas_total = (gas_total_candidate as f64) / (gas_total_reference as f64);

    Results::new(
        size_best,
        size_worst,
        size_total,
        size_negatives,
        size_positives,
        cycles_best,
        cycles_worst,
        cycles_total,
        cycles_negatives,
        cycles_positives,
        ergs_best,
        ergs_worst,
        ergs_total,
        ergs_negatives,
        ergs_positives,
        gas_best,
        gas_worst,
        gas_total,
        gas_negatives,
        gas_positives,
    )
}
