//!
//! The benchmark group representation.
//!

pub mod element;
pub mod results;

use std::collections::BTreeMap;

use serde::Deserialize;
use serde::Serialize;

use crate::benchmark::Benchmark;

use self::element::Element;
use self::results::Results;

///
/// The benchmark group representation.
///
#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct Group {
    /// The group elements.
    pub elements: BTreeMap<String, Element>,
}

impl Group {
    ///
    /// Compares two benchmark groups.
    ///
    pub fn compare<'a>(reference: &'a Self, candidate: &'a Self) -> Results<'a> {
        let elements_number = reference.elements.len();

        let mut size_factors = Vec::with_capacity(elements_number);
        let mut size_min = 1.0;
        let mut size_max = 1.0;
        let mut size_negatives = Vec::with_capacity(elements_number);
        let mut size_positives = Vec::with_capacity(elements_number);
        let mut size_total_reference: u64 = 0;
        let mut size_total_candidate: u64 = 0;

        let mut cycles_factors = Vec::with_capacity(elements_number);
        let mut cycles_min = 1.0;
        let mut cycles_max = 1.0;
        let mut cycles_negatives = Vec::with_capacity(elements_number);
        let mut cycles_positives = Vec::with_capacity(elements_number);
        let mut cycles_total_reference: u64 = 0;
        let mut cycles_total_candidate: u64 = 0;

        let mut ergs_factors = Vec::with_capacity(elements_number);
        let mut ergs_min = 1.0;
        let mut ergs_max = 1.0;
        let mut ergs_negatives = Vec::with_capacity(elements_number);
        let mut ergs_positives = Vec::with_capacity(elements_number);
        let mut ergs_total_reference: u64 = 0;
        let mut ergs_total_candidate: u64 = 0;

        let mut gas_factors = Vec::with_capacity(elements_number);
        let mut gas_min = 1.0;
        let mut gas_max = 1.0;
        let mut gas_negatives = Vec::with_capacity(elements_number);
        let mut gas_positives = Vec::with_capacity(elements_number);
        let mut gas_total_reference: u64 = 0;
        let mut gas_total_candidate: u64 = 0;

        for (path, reference) in reference.elements.iter() {
            if path.contains("tests/solidity/complex/interpreter/test.json")
                && path.contains("#deployer")
            {
                continue;
            }
            let candidate = match candidate.elements.get(path.as_str()) {
                Some(candidate) => candidate,
                None => continue,
            };

            cycles_total_reference += reference.cycles as u64;
            cycles_total_candidate += candidate.cycles as u64;
            let cycles_factor = (candidate.cycles as f64) / (reference.cycles as f64);
            if cycles_factor > 1.0 {
                cycles_negatives.push((cycles_factor, path.as_str()));
            }
            if cycles_factor < 1.0 {
                cycles_positives.push((cycles_factor, path.as_str()));
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
                ergs_negatives.push((ergs_factor, path.as_str()));
            }
            if ergs_factor < 1.0 {
                ergs_positives.push((ergs_factor, path.as_str()));
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
                gas_negatives.push((gas_factor, path.as_str()));
            }
            if gas_factor < 1.0 {
                gas_positives.push((gas_factor, path.as_str()));
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
                size_negatives.push((size_factor, path.as_str()));
            }
            if size_factor < 1.0 {
                size_positives.push((size_factor, path.as_str()));
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

    ///
    /// Returns the EVM interpreter ergs/gas ratio.
    ///
    pub fn evm_interpreter_ratios(&self) -> Vec<(String, f64)> {
        let mut results = Vec::with_capacity(Benchmark::EVM_OPCODES.len());
        for evm_opcode in Benchmark::EVM_OPCODES.into_iter() {
            let name_substring = format!("test.json::{evm_opcode}[");
            let [full, template]: [Element; 2] = self
                .elements
                .iter()
                .filter(|element| element.0.contains(name_substring.as_str()))
                .rev()
                .take(2)
                .map(|element| (element.1.to_owned()))
                .collect::<Vec<Element>>()
                .try_into()
                .expect("Always valid");

            let ergs_difference = full.ergs - template.ergs;
            let gas_difference = full.gas - template.gas;
            let ergs_gas_ratio = (ergs_difference as f64) / (gas_difference as f64);
            results.push((evm_opcode.to_owned(), ergs_gas_ratio));
        }
        results
    }
}
