//!
//! The test directory file system entity.
//!

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::ignore_file::Entity as IgnoreFileEntity;

///
/// The test directory file system entity.
///
#[derive(Debug)]
pub struct FSEntity {
    /// The directory entries. Is `None` for files.
    entries: Option<HashMap<String, FSEntity>>,
}

impl FSEntity {
    ///
    /// Indexes the specified directory.
    ///
    pub fn index(path: &Path, extension: &str) -> anyhow::Result<Self> {
        let mut entries = HashMap::new();

        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let path = entry.path();
            let entry_type = entry.file_type()?;

            if entry_type.is_dir() {
                entries.insert(
                    path.file_stem()
                        .ok_or_else(|| anyhow::anyhow!("Failed to get filename"))?
                        .to_string_lossy()
                        .to_string(),
                    Self::index(&path, extension)?,
                );
                continue;
            }

            if !entry_type.is_file() {
                anyhow::bail!("Invalid entry type");
            }

            if let Some(file_extension) = path.extension() {
                if file_extension != extension {
                    continue;
                }
            } else {
                continue;
            }

            entries.insert(
                path.file_stem()
                    .ok_or_else(|| anyhow::anyhow!("Failed to get filename"))?
                    .to_string_lossy()
                    .to_string(),
                Self { entries: None },
            );
        }
        Ok(Self {
            entries: Some(entries),
        })
    }

    ///
    /// Returns the test names.
    ///
    pub fn as_string_vec(&self, ignored: Option<&IgnoreFileEntity>, flatten: bool) -> Vec<String> {
        let mut accumulator = Vec::with_capacity(16384);
        self.as_string_vec_recursive("", ignored, &mut accumulator, flatten);
        accumulator.sort_by_key(|test| test.to_owned());
        accumulator
    }

    ///
    /// Inner names accumulator function.
    ///
    fn as_string_vec_recursive(
        &self,
        current: &str,
        ignored: Option<&IgnoreFileEntity>,
        accumulator: &mut Vec<String>,
        flatten: bool,
    ) {
        let entries = match &self.entries {
            Some(entries) => entries,
            None => {
                accumulator.push(current.to_owned());
                return;
            }
        };

        for (name, entity) in entries.iter() {
            let ignored = match ignored {
                Some(IgnoreFileEntity::Directory(entries)) => entries.get(name),
                _ => None,
            };
            if let Some(IgnoreFileEntity::Ignored(_)) = ignored {
                continue;
            }
            if flatten && entity.entries.is_none() {
                accumulator.push(current.to_owned());
                continue;
            }
            let mut current = current.to_owned();
            if !current.is_empty() {
                current.push('/');
            }
            current.push_str(name);
            entity.as_string_vec_recursive(&current, ignored, accumulator, flatten);
        }
    }
}
