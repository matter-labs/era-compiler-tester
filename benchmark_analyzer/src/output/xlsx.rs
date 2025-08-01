//!
//! Serializing benchmark data to Excel spreadsheets.
//!

use std::collections::HashMap;

use crate::model::benchmark::Benchmark;

///
/// XLSX output format for benchmark data.
///
#[derive(Default)]
pub struct Xlsx {
    /// Worksheet for runtime gas measurements.
    pub runtime_gas_worksheet: rust_xlsxwriter::Worksheet,
    /// Rows in the runtime gas worksheet.
    pub runtime_gas_rows: HashMap<String, u32>,

    /// Worksheet for deployment gas measurements.
    pub deploy_gas_worksheet: rust_xlsxwriter::Worksheet,
    /// Rows in the deployment gas worksheet.
    pub deploy_gas_rows: HashMap<String, u32>,

    /// Worksheet for runtime bytecode size measurements.
    pub runtime_size_worksheet: rust_xlsxwriter::Worksheet,
    /// Rows in the runtime bytecode size worksheet.
    pub runtime_size_rows: HashMap<String, u32>,

    /// Worksheet for deploy bytecode size measurements.
    pub deploy_size_worksheet: rust_xlsxwriter::Worksheet,
    /// Rows in the deploy bytecode size worksheet.
    pub deploy_size_rows: HashMap<String, u32>,

    /// Toolchain identifiers used to allocate columns.
    pub toolchain_ids: HashMap<String, u16>,
}

impl Xlsx {
    ///
    /// Creates a new XLSX workbook.
    ///
    pub fn new() -> anyhow::Result<Self> {
        let mut runtime_gas_worksheet = rust_xlsxwriter::Worksheet::new();
        runtime_gas_worksheet.set_name("Runtime Gas")?;

        let mut deploy_gas_worksheet = rust_xlsxwriter::Worksheet::new();
        deploy_gas_worksheet.set_name("Deploy Gas")?;

        let mut runtime_size_worksheet = rust_xlsxwriter::Worksheet::new();
        runtime_size_worksheet.set_name("Runtime Size")?;

        let mut deploy_size_worksheet = rust_xlsxwriter::Worksheet::new();
        deploy_size_worksheet.set_name("Deploy Size")?;

        for (worksheet, column_width) in [
            (&mut runtime_gas_worksheet, 100),
            (&mut deploy_gas_worksheet, 100),
            (&mut runtime_size_worksheet, 30),
            (&mut deploy_size_worksheet, 30),
        ] {
            worksheet.write_with_format(
                0,
                0,
                worksheet.name(),
                &Self::worksheet_caption_format(),
            )?;
            worksheet.set_column_width(0, column_width)?;
        }

        Ok(Self {
            runtime_gas_worksheet,
            runtime_gas_rows: HashMap::new(),
            deploy_gas_worksheet,
            deploy_gas_rows: HashMap::new(),
            runtime_size_worksheet,
            runtime_size_rows: HashMap::new(),
            deploy_size_worksheet,
            deploy_size_rows: HashMap::new(),
            toolchain_ids: HashMap::new(),
        })
    }

    ///
    /// Selects mutable references to a worksheet data.
    ///
    pub fn select_worksheet_mut(
        &mut self,
        sheet_name: &str,
    ) -> (&mut rust_xlsxwriter::Worksheet, &mut HashMap<String, u32>) {
        match sheet_name {
            "Runtime Gas" => (&mut self.runtime_gas_worksheet, &mut self.runtime_gas_rows),
            "Deploy Gas" => (&mut self.deploy_gas_worksheet, &mut self.deploy_gas_rows),
            "Runtime Size" => (
                &mut self.runtime_size_worksheet,
                &mut self.runtime_size_rows,
            ),
            "Deploy Size" => (&mut self.deploy_size_worksheet, &mut self.deploy_size_rows),
            _ => panic!("Unknown worksheet name: {sheet_name}"),
        }
    }

    ///
    /// Adds a new column for a toolchain.
    ///
    pub fn add_toolchain_column(&mut self, toolchain_name: &str) -> anyhow::Result<u16> {
        if let Some(toolchain_id) = self.toolchain_ids.get(toolchain_name) {
            return Ok(*toolchain_id);
        }

        let toolchain_id = self.toolchain_ids.len() as u16;
        self.toolchain_ids
            .insert(toolchain_name.to_owned(), toolchain_id);

        let column_toolchain_name = toolchain_name.replace('-', "\n");

        for worksheet in [
            &mut self.runtime_gas_worksheet,
            &mut self.deploy_gas_worksheet,
            &mut self.runtime_size_worksheet,
            &mut self.deploy_size_worksheet,
        ] {
            worksheet.set_column_width(1 + toolchain_id, 12)?;
            worksheet.write_with_format(
                0,
                1 + toolchain_id,
                column_toolchain_name.clone(),
                &Self::column_header_format(),
            )?;
        }

        Ok(toolchain_id)
    }

    ///
    /// Adds a new row for a test.
    ///
    pub fn add_test_row(&mut self, sheet_name: &str, test_name: &str) -> anyhow::Result<u32> {
        let (worksheet, rows) = self.select_worksheet_mut(sheet_name);
        if let Some(index) = rows.get(test_name).copied() {
            return Ok(index);
        }

        let row_index = (rows.len() as u32) + 1;
        rows.insert(test_name.to_owned(), row_index);

        worksheet.write_with_format(
            row_index,
            0,
            test_name.to_owned(),
            &Self::row_header_format(),
        )?;

        Ok(row_index)
    }

    ///
    /// Sets totals for each column in the worksheet.
    ///
    pub fn set_totals(&mut self, sheet_name: &str) -> anyhow::Result<()> {
        let column_count = 1 + self.toolchain_ids.len() * 2;
        let (worksheet, rows) = self.select_worksheet_mut(sheet_name);
        if rows.is_empty() {
            return Ok(());
        }

        let next_row_index = rows.len() as u32;
        worksheet.write_with_format(
            next_row_index,
            0,
            "Total",
            &Self::row_header_summary_format(),
        )?;
        for column_index in 1..column_count {
            let column_name = b'A' + (column_index as u8);
            let formula = format!("SUM({0}2:{0}{1})", column_name as char, next_row_index);
            worksheet.write_formula_with_format(
                next_row_index,
                column_index as u16,
                formula.as_str(),
                &Self::value_format(),
            )?;
        }

        Ok(())
    }

    ///
    /// Sets diffs for the first two data columns in the worksheet.
    ///
    pub fn set_diffs(&mut self, sheet_name: &str) -> anyhow::Result<()> {
        let (worksheet, rows) = self.select_worksheet_mut(sheet_name);

        let column_identifier = "-%";
        worksheet.write_with_format(0, 3, column_identifier, &Self::column_header_format())?;
        worksheet.set_column_width(3, 10)?;

        for row_id in 0..rows.len() + 1 {
            worksheet.write_formula_with_format(
                (row_id + 1) as u32,
                3,
                format!("((C{0}/B{0})-1)*100", row_id + 2).as_str(),
                &Self::percent_format(),
            )?;
        }

        Ok(())
    }

    ///
    /// Returns the final workbook with all worksheets.
    ///
    pub fn finalize(self) -> rust_xlsxwriter::Workbook {
        let mut workbook = rust_xlsxwriter::Workbook::new();
        workbook.push_worksheet(self.runtime_gas_worksheet);
        workbook.push_worksheet(self.deploy_gas_worksheet);
        workbook.push_worksheet(self.runtime_size_worksheet);
        workbook.push_worksheet(self.deploy_size_worksheet);
        workbook
    }

    ///
    /// Returns the eponymous cell format.
    ///
    fn worksheet_caption_format() -> rust_xlsxwriter::Format {
        let format = rust_xlsxwriter::Format::new();
        let format = format.set_bold();
        let format = format.set_font_size(24);
        let format = format.set_font_color("#FFFFFF");
        let format = format.set_background_color("#4C6EF5");
        let format = format.set_align(rust_xlsxwriter::FormatAlign::Center);
        let format = format.set_align(rust_xlsxwriter::FormatAlign::VerticalCenter);
        let format = format.set_border(rust_xlsxwriter::FormatBorder::None);
        format
    }

    ///
    /// Returns the eponymous cell format.
    ///
    fn column_header_format() -> rust_xlsxwriter::Format {
        let format = rust_xlsxwriter::Format::new();
        let format = format.set_bold();
        let format = format.set_font_size(14);
        let format = format.set_font_color("#1E1E1E");
        let format = format.set_background_color("#EEF3FF");
        let format = format.set_align(rust_xlsxwriter::FormatAlign::Center);
        let format = format.set_align(rust_xlsxwriter::FormatAlign::Top);
        let format = format.set_border(rust_xlsxwriter::FormatBorder::None);
        format
    }

    ///
    /// Returns the eponymous cell format.
    ///
    fn row_header_format() -> rust_xlsxwriter::Format {
        let format = rust_xlsxwriter::Format::new();
        let format = format.set_font_size(12);
        let format = format.set_font_color("#1E1E1E");
        let format = format.set_background_color("#DDE6FF");
        let format = format.set_align(rust_xlsxwriter::FormatAlign::Left);
        let format = format.set_border(rust_xlsxwriter::FormatBorder::None);
        format
    }

    ///
    /// Returns the eponymous cell format.
    ///
    fn row_header_summary_format() -> rust_xlsxwriter::Format {
        let format = Self::row_header_format();
        let format = format.set_font_size(16);
        let format = format.set_bold();
        let format = format.set_align(rust_xlsxwriter::FormatAlign::Right);
        format
    }

    ///
    /// Returns the eponymous cell format.
    ///
    fn value_format() -> rust_xlsxwriter::Format {
        let format = rust_xlsxwriter::Format::new();
        let format = format.set_font_size(12);
        let format = format.set_font_color("#000000");
        let format = format.set_background_color("#FFFFFF");
        let format = format.set_align(rust_xlsxwriter::FormatAlign::Right);
        let format = format.set_border(rust_xlsxwriter::FormatBorder::None);
        format
    }

    ///
    /// Returns the eponymous cell format.
    ///
    fn percent_format() -> rust_xlsxwriter::Format {
        let format = rust_xlsxwriter::Format::new();
        let format = format.set_font_size(12);
        let format = format.set_font_color("#000000");
        let format = format.set_background_color("#FFFFFF");
        let format = format.set_align(rust_xlsxwriter::FormatAlign::Right);
        let format = format.set_border(rust_xlsxwriter::FormatBorder::None);
        let format = format.set_num_format("0.000");
        format
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
            let row_identifier = test.metadata.selector.xlsx_identifier();

            for (toolchain_name, toolchain_group) in test.toolchain_groups.into_iter() {
                for codegen_group in toolchain_group.codegen_groups.into_values() {
                    for version_group in codegen_group.versioned_groups.into_values() {
                        for optimization_group in version_group.executables.into_values() {
                            let toolchain_id =
                                xlsx.add_toolchain_column(toolchain_name.as_str())?;

                            if is_deployer {
                                if test.non_zero_gas_values > 0 {
                                    let deployment_gas_row_index = xlsx.add_test_row(
                                        xlsx.deploy_gas_worksheet.name().as_str(),
                                        row_identifier.as_str(),
                                    )?;
                                    xlsx.deploy_gas_worksheet.write_with_format(
                                        deployment_gas_row_index,
                                        1 + toolchain_id,
                                        optimization_group.run.average_gas(),
                                        &Self::value_format(),
                                    )?;
                                }
                            } else {
                                let runtime_gas_row_index = xlsx.add_test_row(
                                    xlsx.runtime_gas_worksheet.name().as_str(),
                                    row_identifier.as_str(),
                                )?;
                                xlsx.runtime_gas_worksheet.write_with_format(
                                    runtime_gas_row_index,
                                    1 + toolchain_id,
                                    optimization_group.run.average_gas(),
                                    &Self::value_format(),
                                )?;
                            }
                            if !optimization_group.run.size.is_empty() {
                                let size_row_index = xlsx.add_test_row(
                                    xlsx.deploy_size_worksheet.name().as_str(),
                                    row_identifier.as_str(),
                                )?;
                                xlsx.deploy_size_worksheet.write_with_format(
                                    size_row_index,
                                    1 + toolchain_id,
                                    optimization_group.run.average_size(),
                                    &Self::value_format(),
                                )?;
                            }
                            if !optimization_group.run.runtime_size.is_empty() {
                                let size_row_index = xlsx.add_test_row(
                                    xlsx.runtime_size_worksheet.name().as_str(),
                                    row_identifier.as_str(),
                                )?;
                                xlsx.runtime_size_worksheet.write_with_format(
                                    size_row_index,
                                    1 + toolchain_id,
                                    optimization_group.run.average_runtime_size(),
                                    &Self::value_format(),
                                )?;
                            }
                        }
                    }
                }
            }
        }

        // xlsx.set_totals("Runtime Gas")?;
        // xlsx.set_totals("Deployment Gas")?;
        // xlsx.set_totals("Bytecode Size")?;

        // if xlsx.next_column_index >= 3 {
        //     xlsx.set_diffs("Runtime Gas")?;
        //     xlsx.set_diffs("Deployment Gas")?;
        //     xlsx.set_diffs("Bytecode Size")?;
        // }

        Ok(xlsx)
    }
}
