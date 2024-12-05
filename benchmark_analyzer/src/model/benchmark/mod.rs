//!
//! The benchmark representation.
//!

pub mod metadata;
pub mod test;

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::Deserialize;
use serde::Serialize;

use metadata::Metadata;

use self::test::Test;
use crate::format::IBenchmarkSerializer;

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

impl Benchmark {
    ///
    /// Writes the benchmark results to a file using a provided serializer.
    ///
    pub fn write_to_file(
        self,
        path: PathBuf,
        serializer: impl IBenchmarkSerializer,
    ) -> anyhow::Result<()> {
        let contents = serializer.serialize_to_string(&self).expect("Always valid");
        std::fs::write(path.as_path(), contents)
            .map_err(|error| anyhow::anyhow!("Benchmark file {path:?} reading: {error}"))?;
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
