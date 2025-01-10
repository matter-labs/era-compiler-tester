//!
//! The benchmark representation.
//!

pub mod metadata;
pub mod test;

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::Deserialize;
use serde::Serialize;

use crate::output::comparison_result::Output;
use crate::output::file::File;
use crate::output::IBenchmarkSerializer;

use metadata::Metadata;

use self::test::Test;

///
/// The benchmark representation.
///
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Benchmark {
    /// Metadata related to the whole benchmark.
    pub metadata: Metadata,
    /// The tests.
    pub tests: BTreeMap<String, Test>,
}

///
/// Writes the benchmark results to a file using a provided serializer.
///
pub fn write_to_file(
    benchmark: &Benchmark,
    path: PathBuf,
    serializer: impl IBenchmarkSerializer,
) -> anyhow::Result<()> {
    match serializer
        .serialize_to_string(benchmark)
        .expect("Always valid")
    {
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
