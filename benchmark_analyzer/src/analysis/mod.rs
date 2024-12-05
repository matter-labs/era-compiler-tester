//!
//! Provides tools for collecting and comparing benchmark results.
//!

pub mod evm_interpreter;

use std::collections::BTreeMap;

use evm_interpreter::is_evm_interpreter_cycles_tests_group;
use evm_interpreter::opcode_cost_ratios;

use crate::model::benchmark::test::codegen::versioned::executable::run::Run;
use crate::model::benchmark::test::metadata::Metadata as TestMetadata;
use crate::model::benchmark::Benchmark;
use crate::results::group::Group;
use crate::results::Results;
use crate::util::btreemap::cross_join_filter_map;
use crate::util::btreemap::intersect_keys;
use crate::util::btreemap::intersect_map;

type GroupRuns<'a> = BTreeMap<&'a str, (&'a TestMetadata, &'a Run)>;

///
/// Collects measurements from a benchmark into groups.
/// Groups may intersect.
///
fn collect_runs(benchmark: &Benchmark) -> BTreeMap<Group<'_>, GroupRuns> {
    let mut result: BTreeMap<Group<'_>, GroupRuns> = BTreeMap::new();

    for (test_identifier, test) in &benchmark.tests {
        for (codegen, codegen_group) in &test.codegen_groups {
            for versioned_group in codegen_group.versioned_groups.values() {
                for (mode, executable) in &versioned_group.executables {
                    for tag in test
                        .metadata
                        .tags
                        .iter()
                        .map(Some)
                        .chain(std::iter::once(None))
                    {
                        let tag = tag.map(|t| t.as_str());
                        result
                            .entry(Group::from_tag(tag, Some(codegen), Some(mode)))
                            .or_default()
                            .insert(test_identifier.as_str(), (&test.metadata, &executable.run));
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
        .map(|(group_name, reference_tests, candidate_tests)| {
            let ratios = if is_evm_interpreter_cycles_tests_group(&group_name) {
                Some((
                    opcode_cost_ratios(&reference_tests),
                    opcode_cost_ratios(&candidate_tests),
                ))
            } else {
                None
            };

            let runs: Vec<(&TestMetadata, &Run, &Run)> = intersect_map(
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
fn compare_runs<'a>(runs: Vec<(&'a TestMetadata, &'a Run, &'a Run)>) -> Results<'a> {
    let elements_number = runs.len();

    let mut size_factors = Vec::with_capacity(elements_number);
    let mut size_min = 1.0;
    let mut size_max = 1.0;
    let mut size_negatives: Vec<(f64, &TestMetadata)> = Vec::with_capacity(elements_number);
    let mut size_positives: Vec<(f64, &TestMetadata)> = Vec::with_capacity(elements_number);
    let mut size_total_reference: u64 = 0;
    let mut size_total_candidate: u64 = 0;

    let mut cycles_factors = Vec::with_capacity(elements_number);
    let mut cycles_min = 1.0;
    let mut cycles_max = 1.0;
    let mut cycles_negatives: Vec<(f64, &TestMetadata)> = Vec::with_capacity(elements_number);
    let mut cycles_positives: Vec<(f64, &TestMetadata)> = Vec::with_capacity(elements_number);
    let mut cycles_total_reference: u64 = 0;
    let mut cycles_total_candidate: u64 = 0;

    let mut ergs_factors = Vec::with_capacity(elements_number);
    let mut ergs_min = 1.0;
    let mut ergs_max = 1.0;
    let mut ergs_negatives: Vec<(f64, &TestMetadata)> = Vec::with_capacity(elements_number);
    let mut ergs_positives: Vec<(f64, &TestMetadata)> = Vec::with_capacity(elements_number);
    let mut ergs_total_reference: u64 = 0;
    let mut ergs_total_candidate: u64 = 0;

    let mut gas_factors = Vec::with_capacity(elements_number);
    let mut gas_min = 1.0;
    let mut gas_max = 1.0;
    let mut gas_negatives = Vec::with_capacity(elements_number);
    let mut gas_positives = Vec::with_capacity(elements_number);
    let mut gas_total_reference: u64 = 0;
    let mut gas_total_candidate: u64 = 0;

    for (metadata, reference, candidate) in runs {
        let file_path = &metadata.selector.path;
        // FIXME: ad-hoc patch
        if file_path.contains(crate::model::evm_interpreter::TEST_PATH) {
            if let Some(input) = &metadata.selector.input {
                if input.is_deployer() {
                    continue;
                }
            }
        }

        cycles_total_reference += reference.cycles as u64;
        cycles_total_candidate += candidate.cycles as u64;
        let cycles_factor = (candidate.cycles as f64) / (reference.cycles as f64);
        if cycles_factor > 1.0 {
            cycles_negatives.push((cycles_factor, metadata));
        }
        if cycles_factor < 1.0 {
            cycles_positives.push((cycles_factor, metadata));
        }
        if cycles_factor < cycles_min {
            cycles_min = cycles_factor;
        }
        if cycles_factor > cycles_max {
            cycles_max = cycles_factor;
        }
        cycles_factors.push(cycles_factor);

        ergs_total_reference += reference.ergs;
        ergs_total_candidate += candidate.ergs;
        let ergs_factor = (candidate.ergs as f64) / (reference.ergs as f64);
        if ergs_factor > 1.0 {
            ergs_negatives.push((ergs_factor, metadata));
        }
        if ergs_factor < 1.0 {
            ergs_positives.push((ergs_factor, metadata));
        }
        if ergs_factor < ergs_min {
            ergs_min = ergs_factor;
        }
        if ergs_factor > ergs_max {
            ergs_max = ergs_factor;
        }
        ergs_factors.push(ergs_factor);

        gas_total_reference += reference.gas;
        gas_total_candidate += candidate.gas;
        let gas_factor = (candidate.gas as f64) / (reference.gas as f64);
        if gas_factor > 1.0 {
            gas_negatives.push((gas_factor, metadata));
        }
        if gas_factor < 1.0 {
            gas_positives.push((gas_factor, metadata));
        }
        if gas_factor < gas_min {
            gas_min = gas_factor;
        }
        if gas_factor > gas_max {
            gas_max = gas_factor;
        }
        gas_factors.push(gas_factor);

        let reference_size = match reference.size {
            Some(size) => size,
            None => continue,
        };
        let candidate_size = match candidate.size {
            Some(size) => size,
            None => continue,
        };
        size_total_reference += reference_size as u64;
        size_total_candidate += candidate_size as u64;
        let size_factor = (candidate_size as f64) / (reference_size as f64);
        if size_factor > 1.0 {
            size_negatives.push((size_factor, metadata));
        }
        if size_factor < 1.0 {
            size_positives.push((size_factor, metadata));
        }
        if size_factor < size_min {
            size_min = size_factor;
        }
        if size_factor > size_max {
            size_max = size_factor;
        }
        size_factors.push(size_factor);
    }

    let size_total = (size_total_candidate as f64) / (size_total_reference as f64);

    let cycles_total = (cycles_total_candidate as f64) / (cycles_total_reference as f64);

    let ergs_total = (ergs_total_candidate as f64) / (ergs_total_reference as f64);

    let gas_total = (gas_total_candidate as f64) / (gas_total_reference as f64);

    Results::new(
        size_min,
        size_max,
        size_total,
        size_negatives,
        size_positives,
        cycles_min,
        cycles_max,
        cycles_total,
        cycles_negatives,
        cycles_positives,
        ergs_min,
        ergs_max,
        ergs_total,
        ergs_negatives,
        ergs_positives,
        gas_min,
        gas_max,
        gas_total,
        gas_negatives,
        gas_positives,
    )
}
