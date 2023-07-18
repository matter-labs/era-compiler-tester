//!
//! The Matter Labs compiler tests directory.
//!

pub mod test;

use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::sync::Mutex;

use crate::filters::Filters;
use crate::summary::Summary;

use super::TestsDirectory;

use self::test::MatterLabsTest;

///
/// The Matter Labs compiler tests directory.
///
pub struct MatterLabsDirectory;

impl TestsDirectory for MatterLabsDirectory {
    type Test = MatterLabsTest;

    fn all_tests(
        directory_path: &Path,
        extension: &'static str,
        summary: Arc<Mutex<Summary>>,
        filters: &Filters,
    ) -> anyhow::Result<Vec<Self::Test>> {
        let mut tests = Vec::new();

        for entry in fs::read_dir(directory_path)? {
            let entry = entry?;
            let path = entry.path();
            let entry_type = entry.file_type().map_err(|error| {
                anyhow::anyhow!(
                    "Failed to get file(`{}`) type: {}",
                    path.to_string_lossy(),
                    error
                )
            })?;

            if entry_type.is_dir() {
                tests.extend(Self::all_tests(&path, extension, summary.clone(), filters)?);
                continue;
            } else if !entry_type.is_file() {
                anyhow::bail!("Invalid file type: {}", path.to_string_lossy());
            }

            if entry.file_name().to_string_lossy().starts_with('.') {
                continue;
            }

            let file_extension = path.extension().ok_or_else(|| {
                anyhow::anyhow!("Failed to get file extension: {}", path.to_string_lossy())
            })?;
            if file_extension != extension {
                continue;
            }

            if let Some(test) = MatterLabsTest::new(path, summary.clone(), filters) {
                tests.push(test);
            }
        }

        Ok(tests)
    }

    fn single_test(
        directory_path: &Path,
        test_path: &Path,
        extension: &'static str,
        summary: Arc<Mutex<Summary>>,
        filters: &Filters,
    ) -> anyhow::Result<Option<Self::Test>> {
        let file_extension = test_path.extension().ok_or_else(|| {
            anyhow::anyhow!(
                "Failed to get file extension: {}",
                test_path.to_string_lossy()
            )
        })?;
        if file_extension != extension {
            anyhow::bail!("Invalid file extension");
        }

        let mut path = directory_path.to_path_buf();
        path.push(test_path);

        Ok(MatterLabsTest::new(path, summary, filters))
    }
}
