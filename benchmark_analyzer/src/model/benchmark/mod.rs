//!
//! The benchmark representation.
//!

pub mod metadata;
pub mod test;

use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::input::foundry::FoundryReport;

use self::metadata::Metadata;
use self::test::input::Input as TestInput;
use self::test::metadata::Metadata as TestMetadata;
use self::test::selector::Selector as TestSelector;
use self::test::toolchain::codegen::versioned::executable::metadata::Metadata as ExecutableMetadata;
use self::test::toolchain::codegen::versioned::executable::run::Run as ExecutableRun;
use self::test::toolchain::codegen::versioned::executable::Executable;
use self::test::Test;

///
/// The benchmark representation.
///
#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Benchmark {
    /// Metadata related to the whole benchmark.
    pub metadata: Metadata,
    /// The tests.
    pub tests: BTreeMap<String, Test>,
}

impl Benchmark {
    ///
    /// A shortcut constructor to set metadata.
    ///
    pub fn new(metadata: Metadata) -> Self {
        Self {
            metadata,
            tests: BTreeMap::default(),
        }
    }

    ///
    /// Extend the benchmark with a Foundry report
    ///
    pub fn extend_with_foundry(&mut self, foundry_report: FoundryReport) -> anyhow::Result<()> {
        let context =
            self.metadata.context.as_ref().ok_or_else(|| {
                anyhow::anyhow!("Benchmark context is required for Foundry reports")
            })?;

        let codegen = context.codegen.as_deref().unwrap_or("codegen-unknown");
        let optimization = context
            .optimization
            .as_deref()
            .unwrap_or("optimization-unknown");

        for contract_report in foundry_report.data.into_iter() {
            let selector = TestSelector {
                project: foundry_report.project.clone(),
                case: Some(contract_report.contract.to_owned()),
                input: Some(TestInput::Deployer {
                    contract_identifier: contract_report.contract.to_owned(),
                }),
            };
            let name = selector.to_string();

            let test = self
                .tests
                .entry(name)
                .or_insert_with(|| Test::new(TestMetadata::new(selector, vec![])));
            test.toolchain_groups
                .entry(foundry_report.toolchain.clone())
                .or_default()
                .codegen_groups
                .entry(codegen.to_owned())
                .or_default()
                .versioned_groups
                .entry(context.compiler_version.clone())
                .or_default()
                .executables
                .insert(
                    optimization.to_owned(),
                    Executable {
                        metadata: ExecutableMetadata::default(),
                        run: ExecutableRun {
                            size: Some(contract_report.deployment.size),
                            cycles: 0,
                            ergs: 0,
                            gas: contract_report.deployment.gas,
                        },
                    },
                );

            for (index, (function, function_report)) in
                contract_report.functions.into_iter().enumerate()
            {
                let selector = TestSelector {
                    project: foundry_report.project.clone(),
                    case: Some(contract_report.contract.to_owned()),
                    input: Some(TestInput::Runtime {
                        input_index: index + 1,
                        name: function,
                    }),
                };
                let name = selector.to_string();

                let test = self
                    .tests
                    .entry(name)
                    .or_insert_with(|| Test::new(TestMetadata::new(selector, vec![])));
                test.toolchain_groups
                    .entry(foundry_report.toolchain.clone())
                    .or_default()
                    .codegen_groups
                    .entry(codegen.to_owned())
                    .or_default()
                    .versioned_groups
                    .entry(context.compiler_version.clone())
                    .or_default()
                    .executables
                    .insert(
                        optimization.to_owned(),
                        Executable {
                            metadata: ExecutableMetadata::default(),
                            run: ExecutableRun {
                                size: None,
                                cycles: 0,
                                ergs: 0,
                                gas: function_report.mean,
                            },
                        },
                    );
            }
        }

        Ok(())
    }
}

impl TryFrom<PathBuf> for Benchmark {
    type Error = anyhow::Error;

    fn try_from(path: PathBuf) -> Result<Self, Self::Error> {
        let text = std::fs::read_to_string(path.as_path())
            .map_err(|error| anyhow::anyhow!("Benchmark file {path:?} reading: {error}"))?;
        let json: Self = serde_json::from_str(text.as_str())
            .map_err(|error| anyhow::anyhow!("Benchmark file {path:?} parsing: {error}"))?;
        Ok(json)
    }
}
