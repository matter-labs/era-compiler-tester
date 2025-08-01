//!
//! The benchmark representation.
//!

pub mod metadata;
pub mod test;

use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::input::foundry_gas::FoundryGasReport;
use crate::input::foundry_size::FoundrySizeReport;
use crate::input::Input;
use crate::input::Report;

use self::metadata::Metadata;
use self::test::input::Input as TestInput;
use self::test::metadata::Metadata as TestMetadata;
use self::test::selector::Selector as TestSelector;
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
    /// Extend the benchmark data with a generic report.
    ///
    pub fn extend(&mut self, input: Input) -> anyhow::Result<()> {
        let toolchain = input.toolchain;
        let project = input.project;
        match input.data {
            Report::FoundryGas(report) => {
                self.extend_with_foundry_gas_report(toolchain, project, report)?;
            }
            Report::FoundrySize(report) => {
                self.extend_with_foundry_size_report(toolchain, project, report)?;
            }
        }
        Ok(())
    }

    ///
    /// Extend the benchmark data with a Foundry gas report.
    ///
    pub fn extend_with_foundry_gas_report(
        &mut self,
        toolchain: String,
        project: String,
        foundry_report: FoundryGasReport,
    ) -> anyhow::Result<()> {
        let context =
            self.metadata.context.as_ref().ok_or_else(|| {
                anyhow::anyhow!("Benchmark context is required for Foundry reports")
            })?;

        let codegen = context.codegen.as_deref().unwrap_or("codegen-unknown");
        let optimization = context
            .optimization
            .as_deref()
            .unwrap_or("optimization-unknown");

        for contract_report in foundry_report.0.into_iter() {
            let selector = TestSelector {
                project: project.clone(),
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
            let run = test
                .toolchain_groups
                .entry(toolchain.clone())
                .or_default()
                .codegen_groups
                .entry(codegen.to_owned())
                .or_default()
                .versioned_groups
                .entry(context.compiler_version.clone())
                .or_default()
                .executables
                .entry(optimization.to_owned())
                .or_default();
            run.run.gas.push(contract_report.deployment.gas);

            for (index, (function, function_report)) in
                contract_report.functions.into_iter().enumerate()
            {
                let selector = TestSelector {
                    project: project.clone(),
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
                let run = test
                    .toolchain_groups
                    .entry(toolchain.clone())
                    .or_default()
                    .codegen_groups
                    .entry(codegen.to_owned())
                    .or_default()
                    .versioned_groups
                    .entry(context.compiler_version.clone())
                    .or_default()
                    .executables
                    .entry(optimization.to_owned())
                    .or_default();
                run.run.gas.push(function_report.mean);
            }
        }

        Ok(())
    }

    ///
    /// Extend the benchmark data with a Foundry size report.
    ///
    pub fn extend_with_foundry_size_report(
        &mut self,
        toolchain: String,
        project: String,
        foundry_report: FoundrySizeReport,
    ) -> anyhow::Result<()> {
        let context =
            self.metadata.context.as_ref().ok_or_else(|| {
                anyhow::anyhow!("Benchmark context is required for Foundry reports")
            })?;

        let codegen = context.codegen.as_deref().unwrap_or("codegen-unknown");
        let optimization = context
            .optimization
            .as_deref()
            .unwrap_or("optimization-unknown");

        for (contract_name, contract_report) in foundry_report.0.into_iter() {
            let selector = TestSelector {
                project: project.clone(),
                case: Some(contract_name.clone()),
                input: Some(TestInput::Deployer {
                    contract_identifier: contract_name.clone(),
                }),
            };
            let name = selector.to_string();

            let test = self
                .tests
                .entry(name)
                .or_insert_with(|| Test::new(TestMetadata::new(selector, vec![])));
            let run = test
                .toolchain_groups
                .entry(toolchain.clone())
                .or_default()
                .codegen_groups
                .entry(codegen.to_owned())
                .or_default()
                .versioned_groups
                .entry(context.compiler_version.clone())
                .or_default()
                .executables
                .entry(optimization.to_owned())
                .or_default();
            run.run.size.push(contract_report.init_size);
            run.run.runtime_size.push(contract_report.runtime_size);
        }

        Ok(())
    }

    ///
    /// Removes tests with zero deployment gas, that are supposed to be non-deployable contracts.
    ///
    pub fn remove_zero_deploy_gas(&mut self) {
        // let max_toolchain_groups = self.tests
        //     .values()
        //     .map(|test| test.toolchain_groups.len())
        //     .max()
        //     .unwrap_or_default();
        self.tests.retain(|_, test| {
            if test.toolchain_groups.is_empty() {
                return false;
            }
            if !test.is_deploy() {
                return true;
            }
            test.non_zero_gas_values = test
                .toolchain_groups
                .values()
                .filter(|group| {
                    group.codegen_groups.values().any(|codegen_group| {
                        codegen_group
                            .versioned_groups
                            .values()
                            .any(|versioned_group| {
                                versioned_group
                                    .executables
                                    .values()
                                    .any(|executable| executable.run.average_gas() != 0)
                            })
                    })
                })
                .count();
            test.toolchain_groups.values().any(|group| {
                group.codegen_groups.values().any(|codegen_group| {
                    codegen_group
                        .versioned_groups
                        .values()
                        .any(|versioned_group| {
                            versioned_group.executables.values().any(|executable| {
                                executable.run.average_size() != 0
                                    || executable.run.average_runtime_size() != 0
                                    || executable.run.average_gas() != 0
                            })
                        })
                })
            })
        });
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
