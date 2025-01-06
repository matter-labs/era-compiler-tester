//!
//! JSON format compatible with LNT.
//!

pub mod benchmark;
pub mod error;

use std::collections::BTreeMap;
use std::path::PathBuf;

use benchmark::machine::Machine;
use benchmark::run_description::RunDescription;
use benchmark::test_description::TestDescription;
use benchmark::LntBenchmark;
use error::LntSerializationError;

use crate::model::benchmark::test::metadata::Metadata as TestMetadata;
use crate::model::benchmark::test::selector::Selector;
use crate::model::benchmark::test::Test;
use crate::model::benchmark::Benchmark;
use crate::output::format::json::make_json_file;
use crate::output::IBenchmarkSerializer;
use crate::output::Output;

///
/// Serialize the benchmark to a set of JSON files compatible with LNT format.
///
#[derive(Default)]
pub struct JsonLNT;

///
/// Generate the test name for a measurement, containing a unique test identifier.
///
fn test_name(selector: &Selector, version: impl std::fmt::Display) -> String {
    fn shorten_file_name(name: &str) -> String {
        let path_buf = PathBuf::from(name);
        path_buf
            .file_name()
            .expect("Always valid")
            .to_str()
            .expect("Always valid")
            .to_string()
    }
    let Selector { path, case, input } = selector;
    let short_path = shorten_file_name(path);
    let short_input = match input {
        Some(crate::Input::Deployer {
            contract_identifier,
        }) => Some(crate::Input::Deployer {
            contract_identifier: shorten_file_name(contract_identifier),
        }),
        _ => input.clone(),
    };
    format!(
        "{} {version}",
        Selector {
            path: short_path.to_string(),
            case: case.clone(),
            input: short_input,
        }
    )
}

impl IBenchmarkSerializer for JsonLNT {
    type Err = LntSerializationError;

    fn serialize_to_string(&self, benchmark: &Benchmark) -> anyhow::Result<Output, Self::Err> {
        let mut files: BTreeMap<String, LntBenchmark> = Default::default();

        let context = if let Some(context) = &benchmark.metadata.context {
            context
        } else {
            return Err(LntSerializationError::NoContext);
        };

        for Test {
            metadata: TestMetadata { selector, .. },
            codegen_groups,
        } in benchmark.tests.values()
        {
            for (codegen, codegen_group) in codegen_groups {
                for (version, versioned_group) in &codegen_group.versioned_groups {
                    for (
                        optimizations,
                        crate::Executable {
                            run: measurements, ..
                        },
                    ) in &versioned_group.executables
                    {
                        let machine_name = format!("{}-{codegen}-{optimizations}", context.machine);

                        let machine = Machine {
                            name: context.machine.clone(),
                            target: context.target.clone(),
                            optimizations: optimizations.to_owned(),
                            toolchain: context.toolchain.clone(),
                        };
                        let run = RunDescription {
                            start_time: benchmark.metadata.start,
                            end_time: benchmark.metadata.end,
                        };
                        files
                            .entry(machine_name)
                            .or_insert(LntBenchmark {
                                format_version: benchmark.metadata.version.clone(),
                                machine,
                                run,
                                tests: vec![],
                            })
                            .tests
                            .push(TestDescription {
                                name: test_name(selector, version),
                                measurements: measurements.clone(),
                            });
                    }
                }
            }
        }

        Ok(Output::MultipleFiles(
            files
                .iter()
                .map(|(key, value)| make_json_file(key, value))
                .collect(),
        ))
    }
}
