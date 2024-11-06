//!
//! Serializing benchmark data to CSV.
//!

use std::fmt::Write;

use super::Benchmark;
use super::IBenchmarkSerializer;
use crate::benchmark::group::element::Element;

/// Serialize the benchmark to CSV in the following format:
/// "group_name", "element_name", "size_str", "cycles", "ergs", "gas"
#[derive(Default)]
pub struct Csv;

impl IBenchmarkSerializer for Csv {
    type Err = std::fmt::Error;

    fn serialize_to_string(&self, benchmark: &Benchmark) -> Result<String, Self::Err> {
        let mut result = String::with_capacity(estimate_csv_size(benchmark));
        result.push_str(r#""group", "test", "size", "cycles", "ergs", "gas""#);
        result.push('\n');
        for (group_name, group) in &benchmark.groups {
            for (
                element_name,
                Element {
                    size,
                    cycles,
                    ergs,
                    gas,
                },
            ) in &group.elements
            {
                let size_str = size.map_or(String::from(""), |s| format!("{}", s));
                writeln!(
                    &mut result,
                    "\"{group_name}\", \"{element_name}\", {size_str}, {cycles}, {ergs}, {gas}",
                )?;
            }
        }
        Ok(result)
    }
}

fn estimate_csv_line_length() -> usize {
    let number_fields = 4;
    let number_field_estimated_max_length = 15;
    let group_name_estimated_max = 10;
    let test_name_estimated_max = 300;
    group_name_estimated_max
        + test_name_estimated_max
        + number_fields * number_field_estimated_max_length
}

fn estimate_csv_size(benchmark: &Benchmark) -> usize {
    (benchmark.groups.len() + 1) * estimate_csv_line_length()
}
