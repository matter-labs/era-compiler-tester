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
    /// The XLSX workbook.
    pub content: rust_xlsxwriter::Workbook,
}

impl Xlsx {
    ///
    /// Returns the eponymous cell format.
    ///
    fn worksheet_caption_format() -> rust_xlsxwriter::Format {
        let format = rust_xlsxwriter::Format::new();
        let format = format.set_bold();
        let format = format.set_font_size(24);
        let format = format.set_font_color("#FFFFFF");
        let format = format.set_background_color("#4C6EF5");
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
        let format = format.set_align(rust_xlsxwriter::FormatAlign::Center);
        let format = format.set_border(rust_xlsxwriter::FormatBorder::None);
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
        let mut workbook = rust_xlsxwriter::Workbook::new();

        let mut gas_worksheet = rust_xlsxwriter::Worksheet::new();
        gas_worksheet.set_name("Runtime Gas")?;
        gas_worksheet.write_with_format(
            0,
            0,
            gas_worksheet.name(),
            &Self::worksheet_caption_format(),
        )?;

        let mut columns = HashMap::new();
        let mut rows = HashMap::new();
        let mut next_column_index = 1;
        let mut next_row_index = 1;

        for test in benchmark.tests.into_values() {
            let row_identifier = format!(
                "{}:{}",
                test.metadata.selector.case.unwrap_or_default(),
                test.metadata
                    .selector
                    .input
                    .map(|input| input.to_string())
                    .unwrap_or_default()
            );
            let row_index = match rows.get(row_identifier.as_str()).copied() {
                Some(index) => index,
                None => {
                    let row_index = next_row_index;
                    rows.insert(row_identifier.clone(), row_index);

                    gas_worksheet.write_with_format(
                        row_index,
                        0,
                        row_identifier.clone(),
                        &Self::row_header_format(),
                    )?;

                    next_row_index += 1;
                    row_index
                }
            };

            for codegen_group in test.codegen_groups.into_values() {
                for version_group in codegen_group.versioned_groups.into_values() {
                    for optimization_group in version_group.executables.into_values() {
                        let column_identifier = test.metadata.selector.domain.clone();
                        let column_index = match columns.get(column_identifier.as_str()).copied() {
                            Some(index) => index,
                            None => {
                                let column_index = next_column_index;
                                columns.insert(column_identifier.clone(), column_index);

                                gas_worksheet.set_column_width(
                                    column_index,
                                    column_identifier.len() as f64,
                                )?;
                                gas_worksheet.write_with_format(
                                    0,
                                    column_index,
                                    column_identifier,
                                    &Self::column_header_format(),
                                )?;

                                next_column_index += 1;
                                column_index
                            }
                        };

                        gas_worksheet.write_with_format(
                            row_index as u32,
                            column_index,
                            optimization_group.run.gas,
                            &Self::value_format(),
                        )?;
                    }
                }
            }
        }

        if next_column_index >= 3 {
            let column_identifier = "-%";
            gas_worksheet.write_with_format(
                0,
                3,
                column_identifier,
                &Self::column_header_format(),
            )?;
            gas_worksheet.set_column_width(3, 10)?;

            for row_id in 0..rows.len() {
                gas_worksheet.write_formula_with_format(
                    (row_id + 1) as u32,
                    3,
                    format!("((C{0}/B{0})-1)*100", row_id + 2).as_str(),
                    &Self::percent_format(),
                )?;
            }
        }

        gas_worksheet.autofit();

        workbook.push_worksheet(gas_worksheet);

        Ok(Self { content: workbook })
    }
}
