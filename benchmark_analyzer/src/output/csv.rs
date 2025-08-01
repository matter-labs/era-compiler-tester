//!
//! Serializing benchmark data to CSV.
//!

use std::fmt::Write;

use crate::model::benchmark::test::metadata::Metadata as TestMetadata;
use crate::model::benchmark::test::selector::Selector;
use crate::model::benchmark::test::Test;
use crate::model::benchmark::Benchmark;

///
/// Serialize the benchmark to CSV in the following format:
/// "group", "codegen", "version", "optimizations", "path", "case", "input", "size", "cycles", "ergs", "gas""
///
#[derive(Default)]
pub struct Csv {
    /// The CSV string.
    pub content: String,
}

impl Csv {
    ///
    /// Estimate the length of a CSV line based on the expected maximum lengths of each field.
    ///
    fn estimate_csv_line_length() -> usize {
        let number_fields = 4;
        let number_field_estimated_max_length = 15;
        let group_name_estimated_max = 10;
        let test_name_estimated_max = 300;
        group_name_estimated_max
            + test_name_estimated_max
            + number_fields * number_field_estimated_max_length
    }

    ///
    /// Estimate the size of the CSV file based on the number of tests and the estimated line length.
    ///
    fn estimate_csv_size(benchmark: &Benchmark) -> usize {
        (benchmark.tests.len() + 1) * Self::estimate_csv_line_length()
    }
}

impl From<Benchmark> for Csv {
    fn from(benchmark: Benchmark) -> Csv {
        let mut content = String::with_capacity(Self::estimate_csv_size(&benchmark));
        content.push_str(
            r#""group", "codegen", "version", "optimizations", "path", "case", "input", "size", "cycles", "ergs", "gas""#,
        );
        content.push('\n');

        for Test {
            metadata:
                TestMetadata {
                    tags,
                    selector:
                        Selector {
                            project: path,
                            case,
                            input,
                            ..
                        },
                },
            toolchain_groups,
            ..
        } in benchmark.tests.into_values()
        {
            for (_toolchain, toolchain_group) in toolchain_groups.into_iter() {
                for (codegen, codegen_group) in toolchain_group.codegen_groups.into_iter() {
                    for (version, versioned_group) in codegen_group.versioned_groups.into_iter() {
                        for (optimizations, crate::Executable { run, .. }) in
                            versioned_group.executables.into_iter()
                        {
                            let tags = {
                                let mut tags = tags.clone();
                                tags.sort();
                                tags.join(" ")
                            };
                            let input = input.clone().map(|s| s.to_string()).unwrap_or_default();
                            let case = case.as_deref().unwrap_or_default();

                            let size = run.average_size();
                            let cycles = run.average_cycles();
                            let ergs = run.average_ergs();
                            let gas = run.average_gas();

                            writeln!(
                                &mut content,
                                r#""{tags}", "{codegen}", "{version}", "{optimizations}", "{path}", "{case}", "{input}", {size}, {cycles}, {ergs}, {gas}"#,
                            ).expect("Always valid");
                        }
                    }
                }
            }
        }

        Self { content }
    }
}
