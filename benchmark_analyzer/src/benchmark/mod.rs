//!
//! The benchmark representation.
//!

pub mod group;

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::Deserialize;
use serde::Serialize;

use self::group::results::Results;
use self::group::Group;

///
/// The benchmark representation.
///
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Benchmark {
    /// The benchmark groups.
    pub groups: BTreeMap<String, Group>,
}

impl Benchmark {
    ///
    /// Compares two benchmarks.
    ///
    pub fn compare<'a>(reference: &'a Self, candidate: &'a Self) -> BTreeMap<&'a str, Results<'a>> {
        let mut results = BTreeMap::new();

        for (group_name, reference) in reference.groups.iter() {
            let candidate = match candidate.groups.get(group_name) {
                Some(candidate) => candidate,
                None => continue,
            };

            let group_results = Group::compare(reference, candidate);
            results.insert(group_name.as_str(), group_results);
        }

        results
    }

    ///
    /// Writes the benchmark to a file.
    ///
    pub fn write_to_file(self, path: PathBuf) -> anyhow::Result<()> {
        let contents = serde_json::to_string(&self).expect("Always valid");
        std::fs::write(path.as_path(), contents)
            .map_err(|error| anyhow::anyhow!("Benchmark file {:?} reading: {}", path, error))?;
        Ok(())
    }
}

impl TryFrom<PathBuf> for Benchmark {
    type Error = anyhow::Error;

    fn try_from(path: PathBuf) -> Result<Self, Self::Error> {
        let text = std::fs::read_to_string(path.as_path())
            .map_err(|error| anyhow::anyhow!("Benchmark file {:?} reading: {}", path, error))?;
        let json: Self = serde_json::from_str(text.as_str())
            .map_err(|error| anyhow::anyhow!("Benchmark file {:?} parsing: {}", path, error))?;
        Ok(json)
    }
}
