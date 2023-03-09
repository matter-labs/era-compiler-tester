//!
//! The coverage watcher library.
//!

pub(crate) mod directory;
pub(crate) mod ignore_file;
pub(crate) mod index;

use std::collections::HashSet;

use crate::index::FSEntity;

pub use self::directory::TestsDirectory;
pub use self::ignore_file::Entity as IgnoreFileEntity;

///
/// The tests directory.
///
pub struct TestsSet {
    /// The name.
    pub name: String,
    /// The first value is a path, the second is an extension.
    pub directories: Vec<TestsDirectory>,
}

impl TestsSet {
    ///
    /// Get missed tests for every group.
    ///
    pub fn get_missed_for_groups(
        groups: Vec<Vec<Self>>,
        ignore_file: &IgnoreFileEntity,
    ) -> anyhow::Result<Vec<(String, Vec<String>)>> {
        let mut result = Vec::new();
        for group in groups {
            result.extend(Self::get_missed(group, ignore_file)?);
        }
        Ok(result)
    }

    ///
    /// Get missed tests for every element of array.
    ///
    fn get_missed(
        tests_sets: Vec<Self>,
        ignore_file: &IgnoreFileEntity,
    ) -> anyhow::Result<Vec<(String, Vec<String>)>> {
        let mut indexes = Vec::with_capacity(tests_sets.len());
        for tests_set in tests_sets.iter() {
            let mut tests_set_indexes = Vec::with_capacity(tests_set.directories.len());
            for directory in tests_set.directories.iter() {
                tests_set_indexes.push(FSEntity::index(
                    directory.path.as_path(),
                    directory.extension.as_str(),
                )?);
            }
            indexes.push(tests_set_indexes);
        }

        let mut result = Vec::new();

        for (index, tests_set) in tests_sets.iter().enumerate() {
            let mut set = HashSet::new();
            for (directory_index, directory) in tests_set.directories.iter().enumerate() {
                set.extend(indexes[index][directory_index].as_string_vec(None, directory.flatten));
            }

            let ignore = ignore_file.get(tests_set.name.as_str());

            let mut missed = Vec::new();
            for (index_other, tests_set_other) in tests_sets.iter().enumerate() {
                if index_other == index {
                    continue;
                }
                for (directory_index, directory) in tests_set_other.directories.iter().enumerate() {
                    for test in indexes[index_other][directory_index]
                        .as_string_vec(ignore, directory.flatten)
                    {
                        if !set.contains(test.as_str()) {
                            missed.push(test);
                        }
                    }
                }
            }
            missed.sort();
            missed.dedup();

            result.push((tests_set.name.clone(), missed));
        }
        Ok(result)
    }
}
