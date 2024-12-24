//!
//! Serializing benchmark data to CSV.
//!

use std::fmt::Write as _;

use crate::format::IBenchmarkSerializer;
use crate::model::benchmark::test::metadata::Metadata as TestMetadata;
use crate::model::benchmark::test::selector::Selector;
use crate::model::benchmark::test::Test;
use crate::model::benchmark::Benchmark;

use super::Output;

///
/// Serialize the benchmark to CSV in the following format:
/// "group", "codegen", "version", "optimizations", "path", "case", "input", "size", "cycles", "ergs", "gas""
///
#[derive(Default)]
pub struct Csv;

impl IBenchmarkSerializer for Csv {
    type Err = std::fmt::Error;

    fn serialize_to_string(&self, benchmark: &Benchmark) -> Result<Output, Self::Err> {
        let mut result = String::with_capacity(estimate_csv_size(benchmark));
        result.push_str(
            r#""group", "codegen", "version", "optimizations", "path", "case", "input", "size", "cycles", "ergs", "gas""#,
        );

        result.push('\n');
        for Test {
            metadata:
                TestMetadata {
                    tags,
                    selector: Selector { path, case, input },
                },
            codegen_groups,
        } in benchmark.tests.values()
        {
            for (codegen, codegen_group) in codegen_groups {
                for (version, versioned_group) in &codegen_group.versioned_groups {
                    for (
                        optimizations,
                        crate::Executable {
                            run:
                                crate::Run {
                                    size,
                                    cycles,
                                    ergs,
                                    gas,
                                },
                            ..
                        },
                    ) in &versioned_group.executables
                    {
                        let tags = {
                            let mut tags = tags.clone();
                            tags.sort();
                            tags.join(" ")
                        };
                        let size_str = size.map(|s| s.to_string()).unwrap_or_default();
                        let input = input.clone().map(|s| s.to_string()).unwrap_or_default();
                        let case = case.as_deref().unwrap_or_default();
                        writeln!(
                            &mut result,
                            r#""{tags}", "{codegen}", "{version}", "{optimizations}", "{path}", "{case}", "{input}", {size_str}, {cycles}, {ergs}, {gas}"#,
                        )?;
                    }
                }
            }
        }
        Ok(Output::SingleFile(result))
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
    (benchmark.tests.len() + 1) * estimate_csv_line_length()
}
