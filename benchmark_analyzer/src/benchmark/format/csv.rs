//!
//! Serializing benchmark data to CSV.
//!

use std::fmt::Write;

use super::Benchmark;
use super::IBenchmarkSerializer;
use crate::benchmark::group::element::selector::Selector;
use crate::benchmark::group::element::Element;
use crate::benchmark::metadata::Metadata;

///
/// Serialize the benchmark to CSV in the following format:
/// "group_name", "element_name", "size_str", "cycles", "ergs", "gas"
///
#[derive(Default)]
pub struct Csv;

impl IBenchmarkSerializer for Csv {
    type Err = std::fmt::Error;

    fn serialize_to_string(&self, benchmark: &Benchmark) -> Result<String, Self::Err> {
        let mut result = String::with_capacity(estimate_csv_size(benchmark));
        result.push_str(
            r#""group", "mode", "version", "path", "case", "input", "size", "cycles", "ergs", "gas""#,
        );
        result.push('\n');
        for (group_name, group) in &benchmark.groups {
            for Element {
                metadata:
                    Metadata {
                        selector: Selector { path, case, input },
                        mode,
                        version,
                        group: _,
                    },
                size,
                cycles,
                ergs,
                gas,
            } in group.elements.values()
            {
                let size_str = size.map_or(String::from(""), |s| s.to_string());
                let mode = mode.as_deref().unwrap_or_default();
                let input = input.clone().map(|s| s.to_string()).unwrap_or_default();
                let case = case.as_deref().unwrap_or_default();
                let version = version.as_deref().unwrap_or_default();
                writeln!(
                    &mut result,
                    r#""{group_name}", "{mode}", "{version}", "{path}", "{case}", "{input}", {size_str}, {cycles}, {ergs}, {gas}"#,
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
