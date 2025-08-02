//!
//! XLSX output format for benchmark data.
//!

pub mod worksheet;

use std::collections::HashMap;

use crate::model::benchmark::Benchmark;

use self::worksheet::Worksheet;

///
/// XLSX output format for benchmark data.
///
#[derive(Default)]
pub struct Xlsx {
    /// Worksheet for runtime gas measurements.
    pub runtime_gas_worksheet: Worksheet,
    /// Worksheet for deployment gas measurements.
    pub deploy_gas_worksheet: Worksheet,
    /// Worksheet for runtime bytecode size measurements.
    pub runtime_size_worksheet: Worksheet,
    /// Worksheet for deploy bytecode size measurements.
    pub deploy_size_worksheet: Worksheet,
    /// Toolchain identifiers used to allocate columns.
    pub toolchain_ids: HashMap<String, u16>,
}

impl Xlsx {
    ///
    /// Creates a new XLSX workbook.
    ///
    pub fn new() -> anyhow::Result<Self> {
        let project_header = ("Project", 15);
        let contract_header = ("Contract", 60);
        let function_header = ("Function", 40);

        let runtime_gas_worksheet = Worksheet::new(
            "Runtime Gas",
            vec![project_header, contract_header, function_header],
        )?;
        let deploy_gas_worksheet =
            Worksheet::new("Deploy Gas", vec![project_header, contract_header])?;
        let runtime_size_worksheet =
            Worksheet::new("Runtime Size", vec![project_header, contract_header])?;
        let deploy_size_worksheet =
            Worksheet::new("Deploy Size", vec![project_header, contract_header])?;

        Ok(Self {
            runtime_gas_worksheet,
            deploy_gas_worksheet,
            runtime_size_worksheet,
            deploy_size_worksheet,
            toolchain_ids: HashMap::new(),
        })
    }

    ///
    /// Allocates a new toolchain ID or returns an existing one.
    ///
    pub fn get_toolchain_id(&mut self, toolchain_name: &str) -> u16 {
        if let Some(toolchain_id) = self.toolchain_ids.get(toolchain_name) {
            return *toolchain_id;
        }

        let toolchain_id = self.toolchain_ids.len() as u16;
        self.toolchain_ids
            .insert(toolchain_name.to_owned(), toolchain_id);
        toolchain_id
    }

    ///
    /// Returns the final workbook with all worksheets.
    ///
    pub fn finalize(self) -> rust_xlsxwriter::Workbook {
        let mut workbook = rust_xlsxwriter::Workbook::new();
        workbook.push_worksheet(self.runtime_gas_worksheet.into_inner());
        workbook.push_worksheet(self.deploy_gas_worksheet.into_inner());
        workbook.push_worksheet(self.runtime_size_worksheet.into_inner());
        workbook.push_worksheet(self.deploy_size_worksheet.into_inner());
        workbook
    }
}

impl TryFrom<Benchmark> for Xlsx {
    type Error = anyhow::Error;

    fn try_from(benchmark: Benchmark) -> Result<Self, Self::Error> {
        let mut xlsx = Self::new()?;

        for test in benchmark.tests.into_values() {
            let is_deployer = test
                .metadata
                .selector
                .input
                .as_ref()
                .map(|input| input.is_deploy())
                .unwrap_or_default();
            let project = test.metadata.selector.project;
            let contract = test
                .metadata
                .selector
                .case
                .unwrap_or_else(|| "Unknown".to_owned());
            let function = test
                .metadata
                .selector
                .input
                .as_ref()
                .and_then(|input| input.runtime_name());

            for (toolchain_name, toolchain_group) in test.toolchain_groups.into_iter() {
                for codegen_group in toolchain_group.codegen_groups.into_values() {
                    for version_group in codegen_group.versioned_groups.into_values() {
                        for optimization_group in version_group.executables.into_values() {
                            let toolchain_id = xlsx.get_toolchain_id(toolchain_name.as_str());

                            if is_deployer {
                                if test.non_zero_gas_values > 0 {
                                    xlsx.deploy_gas_worksheet.add_toolchain_column(
                                        toolchain_name.as_str(),
                                        toolchain_id,
                                    )?;
                                    xlsx.deploy_gas_worksheet.write_test_value(
                                        project.as_str(),
                                        contract.as_str(),
                                        None,
                                        toolchain_id,
                                        optimization_group.run.average_gas(),
                                    )?;
                                }
                            } else {
                                xlsx.runtime_gas_worksheet
                                    .add_toolchain_column(toolchain_name.as_str(), toolchain_id)?;
                                xlsx.runtime_gas_worksheet.write_test_value(
                                    project.as_str(),
                                    contract.as_str(),
                                    function,
                                    toolchain_id,
                                    optimization_group.run.average_gas(),
                                )?;
                            }
                            if !optimization_group.run.size.is_empty() {
                                xlsx.deploy_size_worksheet
                                    .add_toolchain_column(toolchain_name.as_str(), toolchain_id)?;
                                xlsx.deploy_size_worksheet.write_test_value(
                                    project.as_str(),
                                    contract.as_str(),
                                    None,
                                    toolchain_id,
                                    optimization_group.run.average_size(),
                                )?;
                            }
                            if !optimization_group.run.runtime_size.is_empty() {
                                xlsx.runtime_size_worksheet
                                    .add_toolchain_column(toolchain_name.as_str(), toolchain_id)?;
                                xlsx.runtime_size_worksheet.write_test_value(
                                    project.as_str(),
                                    contract.as_str(),
                                    None,
                                    toolchain_id,
                                    optimization_group.run.average_runtime_size(),
                                )?;
                            }
                        }
                    }
                }
            }
        }

        xlsx.runtime_gas_worksheet
            .set_totals(xlsx.toolchain_ids.len())?;
        xlsx.deploy_gas_worksheet
            .set_totals(xlsx.toolchain_ids.len())?;
        xlsx.runtime_size_worksheet
            .set_totals(xlsx.toolchain_ids.len())?;
        xlsx.deploy_size_worksheet
            .set_totals(xlsx.toolchain_ids.len())?;

        // if xlsx.next_column_index >= 3 {
        //     xlsx.set_diffs("Runtime Gas")?;
        //     xlsx.set_diffs("Deployment Gas")?;
        //     xlsx.set_diffs("Bytecode Size")?;
        // }

        Ok(xlsx)
    }
}
