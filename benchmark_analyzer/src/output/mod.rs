//!
//! Benchmark analyzer output.
//!

pub mod csv;
pub mod file;
pub mod json;

use std::path::PathBuf;

use crate::model::benchmark::Benchmark;
use crate::output::csv::Csv;
use crate::output::json::lnt::JsonLNT;
use crate::output::json::Json;
use crate::output_format::OutputFormat;

use self::file::File;

///
/// Result of comparing two benchmarks.
///
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub enum Output {
    /// Benchmark output is a single unnamed file.
    SingleFile(String),
    /// Benchmark output is structured as a file tree, relative to some
    /// user-provided output directory.
    MultipleFiles(Vec<File>),
}

impl Output {
    ///
    /// Writes the benchmark results to a file using a provided serializer.
    ///
    pub fn write_to_file(self, path: PathBuf) -> anyhow::Result<()> {
        match self {
            Output::SingleFile(contents) => {
                std::fs::write(path.as_path(), contents)
                    .map_err(|error| anyhow::anyhow!("Benchmark file {path:?} writing: {error}"))?;
            }
            Output::MultipleFiles(files) => {
                if !files.is_empty() {
                    std::fs::create_dir_all(&path)?;
                }
                for File {
                    path: relative_path,
                    contents,
                } in files
                {
                    let file_path = path.join(relative_path);
                    std::fs::write(file_path.as_path(), contents).map_err(|error| {
                        anyhow::anyhow!("Benchmark file {file_path:?} writing: {error}")
                    })?;
                }
            }
        }
        Ok(())
    }
}

impl TryFrom<(Benchmark, OutputFormat)> for Output {
    type Error = anyhow::Error;

    fn try_from(
        (benchmark, output_format): (Benchmark, OutputFormat),
    ) -> Result<Self, Self::Error> {
        Ok(match output_format {
            OutputFormat::Json => Json::from(benchmark).into(),
            OutputFormat::Csv => Csv::from(benchmark).into(),
            OutputFormat::JsonLNT => JsonLNT::try_from(benchmark)?.into(),
        })
    }
}

impl From<Json> for Output {
    fn from(value: Json) -> Self {
        Output::SingleFile(value.content)
    }
}

impl From<Csv> for Output {
    fn from(value: Csv) -> Self {
        Output::SingleFile(value.content)
    }
}

impl From<JsonLNT> for Output {
    fn from(value: JsonLNT) -> Self {
        Self::MultipleFiles(
            value
                .files
                .iter()
                .map(|(key, value)| File::new(key, value))
                .collect(),
        )
    }
}
