//!
//! The test file update conflicts flags.
//!

use std::fmt::Display;
use std::fmt::Formatter;
use std::fs::File;
use std::io::Read;
use std::path::Path;

use super::test_file::TestFile;

///
/// The test file update conflicts flags.
///
pub struct Conflicts {
    /// The test source data.
    data: bool,
    /// The test enabled flag.
    enabled: bool,
    /// The test group.
    group: bool,
    /// The test comment.
    comment: bool,
    /// The test modes filter.
    modes: bool,
    /// The test version filter.
    version: bool,
}

impl Conflicts {
    ///
    /// Try to get conflicts flags from the old test entity changes.
    ///
    pub fn try_from_test_entity_changes(test: &TestFile, path: &Path) -> anyhow::Result<Self> {
        let hash = test
            .hash
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Test file hash is None"))?;

        let mut file = File::open(path)?;
        let mut data = String::new();
        file.read_to_string(&mut data)
            .map_err(|error| anyhow::anyhow!("Failed to read test file: {}", error))?;
        let actual_hash = TestFile::md5(data.as_str());

        Ok(Self {
            data: !hash.eq(&actual_hash),
            enabled: !test.enabled,
            group: test.group.is_some(),
            comment: test.comment.is_some(),
            modes: test.modes.is_some(),
            version: test.version.is_some(),
        })
    }
}

impl Display for Conflicts {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if !(self.data || self.enabled || self.group || self.comment || self.modes || self.version)
        {
            return write!(f, "no conflicts");
        }

        let mut conflicts = Vec::new();
        if self.data {
            conflicts.push("test source changes");
        }
        if self.enabled {
            conflicts.push("enabled flag");
        }
        if self.group {
            conflicts.push("group");
        }
        if self.comment {
            conflicts.push("comment");
        }
        if self.modes {
            conflicts.push("modes filter");
        }
        if self.version {
            conflicts.push("version filter");
        }
        let conflicts_str = conflicts.join(", ");
        write!(f, "{} ", conflicts_str)?;
        if conflicts.len() == 1 {
            write!(f, "was overwritten by update")
        } else {
            write!(f, "were overwritten by update")
        }
    }
}
