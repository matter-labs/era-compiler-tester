//!
//! XLSX output format for benchmark data.
//!

pub mod worksheet;

use std::collections::HashMap;

use crate::input::source::Source;
use crate::model::benchmark::Benchmark;
use crate::target::Target;

use self::worksheet::Worksheet;

///
/// XLSX output format for benchmark data.
///
#[derive(Default)]
pub struct Xlsx {
    /// Worksheet for runtime fee measurements.
    pub runtime_fee_worksheet: Worksheet,
    /// Worksheet for deployment fee measurements.
    pub deploy_fee_worksheet: Worksheet,
    /// Worksheet for runtime bytecode size measurements.
    pub runtime_size_worksheet: Worksheet,
    /// Worksheet for deploy bytecode size measurements.
    pub deploy_size_worksheet: Worksheet,

    /// Toolchain identifiers.
    pub toolchains: Vec<String>,
    /// Toolchain indexes used to allocate columns.
    pub toolchain_ids: HashMap<String, u16>,

    /// The target platform.
    pub target: Target,
}

impl Xlsx {
    ///
    /// Creates a new XLSX workbook.
    ///
    pub fn new(target: Target) -> anyhow::Result<Self> {
        let project_header = ("Project", 15);
        let contract_header = ("Contract", 60);
        let function_header = ("Function", 40);

        let runtime_fee_caption = match target {
            Target::EVM => "Runtime Gas",
            Target::EraVM => "Runtime Ergs",
        };
        let deploy_fee_caption = match target {
            Target::EVM => "Deploy Gas",
            Target::EraVM => "Deploy Ergs",
        };

        let runtime_fee_worksheet = Worksheet::new(
            runtime_fee_caption,
            vec![project_header, contract_header, function_header],
        )?;
        let deploy_fee_worksheet =
            Worksheet::new(deploy_fee_caption, vec![project_header, contract_header])?;
        let runtime_size_worksheet =
            Worksheet::new("Runtime Size", vec![project_header, contract_header])?;
        let deploy_size_worksheet =
            Worksheet::new("Deploy Size", vec![project_header, contract_header])?;

        Ok(Self {
            runtime_fee_worksheet,
            deploy_fee_worksheet,
            runtime_size_worksheet,
            deploy_size_worksheet,

            toolchains: Vec::with_capacity(8),
            toolchain_ids: HashMap::with_capacity(8),

            target,
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
        self.toolchains.push(toolchain_name.to_owned());
        toolchain_id
    }

    ///
    /// Returns the final workbook with all worksheets.
    ///
    pub fn finalize(self) -> rust_xlsxwriter::Workbook {
        let mut workbook = rust_xlsxwriter::Workbook::new();
        workbook.push_worksheet(self.runtime_fee_worksheet.into_inner());
        workbook.push_worksheet(self.deploy_fee_worksheet.into_inner());
        workbook.push_worksheet(self.runtime_size_worksheet.into_inner());
        if let Target::EVM = self.target {
            workbook.push_worksheet(self.deploy_size_worksheet.into_inner());
        }
        workbook
    }
}

impl TryFrom<(Benchmark, Source, Target)> for Xlsx {
    type Error = anyhow::Error;

    fn try_from(
        (benchmark, source, target): (Benchmark, Source, Target),
    ) -> Result<Self, Self::Error> {
        let mut xlsx = Self::new(target)?;

        'outer: for test in benchmark.tests.into_values() {
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

            let blacklist = vec![(
                "aave-v3",
                "lib/solidity-utils/lib/openzeppelin-contracts-upgradeable/lib/openzeppelin-contracts/contracts/proxy/transparent/TransparentUpgradeableProxy.sol:TransparentUpgradeableProxy",
                "fallback()",
            ), (
                "solady",
                "test/utils/mocks/MockMulticallable.sol:MockMulticallable",
                "multicallBrutalized(bytes[])",
            ), (
                "solady",
                "src/accounts/ERC6551Proxy.sol:ERC6551Proxy",
                "fallback()",
            )];
            for (project_b, contract_b, function_b) in blacklist.into_iter() {
                if project.as_str() == project_b
                    && contract.as_str() == contract_b
                    && function == Some(function_b)
                {
                    continue 'outer;
                }
            }
            if project == "compiler-tester"
                && contract.starts_with("tests/solidity/complex/interpreter/test.json")
            {
                continue 'outer;
            }

            for (toolchain_name, toolchain_group) in test.toolchain_groups.into_iter() {
                for (codegen_name, codegen_group) in toolchain_group.codegen_groups.into_iter() {
                    for (version_name, version_group) in codegen_group.versioned_groups.into_iter()
                    {
                        for (optimization_name, optimization_group) in
                            version_group.executables.into_iter()
                        {
                            let toolchain_name = format!("{toolchain_name}-{version_name}-{codegen_name}-{optimization_name}");
                            let toolchain_id = xlsx.get_toolchain_id(toolchain_name.as_str());

                            if is_deployer {
                                if test.non_zero_gas_values > 0 {
                                    xlsx.deploy_fee_worksheet.add_toolchain_column(
                                        toolchain_name.as_str(),
                                        toolchain_id,
                                    )?;
                                    xlsx.deploy_fee_worksheet.write_test_value(
                                        project.as_str(),
                                        contract.as_str(),
                                        None,
                                        toolchain_id,
                                        match target {
                                            Target::EVM => optimization_group.run.average_gas(),
                                            Target::EraVM => optimization_group.run.average_ergs(),
                                        },
                                    )?;
                                }
                            } else {
                                xlsx.runtime_fee_worksheet
                                    .add_toolchain_column(toolchain_name.as_str(), toolchain_id)?;
                                xlsx.runtime_fee_worksheet.write_test_value(
                                    project.as_str(),
                                    contract.as_str(),
                                    function,
                                    toolchain_id,
                                    match target {
                                        Target::EVM => optimization_group.run.average_gas(),
                                        Target::EraVM => optimization_group.run.average_ergs(),
                                    },
                                )?;
                            }
                            if let Target::EVM = target {
                                if !optimization_group.run.size.is_empty() {
                                    xlsx.deploy_size_worksheet.add_toolchain_column(
                                        toolchain_name.as_str(),
                                        toolchain_id,
                                    )?;
                                    xlsx.deploy_size_worksheet.write_test_value(
                                        project.as_str(),
                                        contract.as_str(),
                                        None,
                                        toolchain_id,
                                        optimization_group.run.average_size(),
                                    )?;
                                }
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

        xlsx.runtime_fee_worksheet
            .set_totals(xlsx.toolchain_ids.len())?;
        xlsx.deploy_fee_worksheet
            .set_totals(xlsx.toolchain_ids.len())?;
        xlsx.runtime_size_worksheet
            .set_totals(xlsx.toolchain_ids.len())?;
        if let Target::EVM = target {
            xlsx.deploy_size_worksheet
                .set_totals(xlsx.toolchain_ids.len())?;
        }

        let comparison_mapping = match (source, target) {
            (Source::Tooling, _) => vec![(6, 4), (7, 5), (6, 2), (7, 3), (6, 0), (7, 1)],
            (Source::CompilerTester, Target::EVM) => vec![(2, 6), (3, 7), (0, 4), (1, 5)],
            (Source::CompilerTester, Target::EraVM) => {
                vec![(4, 10), (5, 11), (0, 6), (1, 7), (2, 8), (3, 9)]
            }
        };

        for (index, (toolchain_id_1, toolchain_id_2)) in comparison_mapping.into_iter().enumerate()
        {
            xlsx.runtime_fee_worksheet.set_diffs(
                toolchain_id_1,
                xlsx.toolchains[toolchain_id_1 as usize].as_str(),
                toolchain_id_2,
                xlsx.toolchains[toolchain_id_2 as usize].as_str(),
                xlsx.toolchain_ids.len() as u16,
                index as u16,
            )?;
            xlsx.deploy_fee_worksheet.set_diffs(
                toolchain_id_1,
                xlsx.toolchains[toolchain_id_1 as usize].as_str(),
                toolchain_id_2,
                xlsx.toolchains[toolchain_id_2 as usize].as_str(),
                xlsx.toolchain_ids.len() as u16,
                index as u16,
            )?;
            xlsx.runtime_size_worksheet.set_diffs(
                toolchain_id_1,
                xlsx.toolchains[toolchain_id_1 as usize].as_str(),
                toolchain_id_2,
                xlsx.toolchains[toolchain_id_2 as usize].as_str(),
                xlsx.toolchain_ids.len() as u16,
                index as u16,
            )?;
            if let Target::EVM = target {
                xlsx.deploy_size_worksheet.set_diffs(
                    toolchain_id_1,
                    xlsx.toolchains[toolchain_id_1 as usize].as_str(),
                    toolchain_id_2,
                    xlsx.toolchains[toolchain_id_2 as usize].as_str(),
                    xlsx.toolchain_ids.len() as u16,
                    index as u16,
                )?;
            }
        }

        Ok(xlsx)
    }
}
