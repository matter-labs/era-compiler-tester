//!
//! The benchmark group representation.
//!

pub mod element;
pub mod results;

use std::collections::BTreeMap;

use serde::Deserialize;
use serde::Serialize;

use self::element::Element;
use self::results::Results;

///
/// The benchmark group representation.
///
#[derive(Debug, Default, Serialize, Deserialize)]
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

        for (path, reference) in reference.elements.iter() {
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

        let size_geomean = math::mean::geometric(size_factors.as_slice());
        let size_total = (size_total_candidate as f64) / (size_total_reference as f64);

        let cycles_geomean = math::mean::geometric(cycles_factors.as_slice());
        let cycles_total = (cycles_total_candidate as f64) / (cycles_total_reference as f64);

        Results::new(
            size_geomean,
            size_min,
            size_max,
            size_total,
            size_negatives,
            size_positives,
            cycles_geomean,
            cycles_min,
            cycles_max,
            cycles_total,
            cycles_negatives,
            cycles_positives,
        )
    }
}
