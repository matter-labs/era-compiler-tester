//!
//! The ignore file entity.
//!

use std::collections::HashMap;

use serde::Deserialize;

///
/// The ignore file entity.
///
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum Entity {
    /// The directory.
    Directory(HashMap<String, Entity>),
    /// The ignored entity.
    Ignored(String),
}

impl Entity {
    ///
    /// Get the subdirectory.
    ///
    pub fn get(&self, path: &str) -> Option<&Self> {
        let mut directory = Some(self);
        for part in path.split('/') {
            directory = match directory {
                Some(Entity::Directory(entries)) => entries.get(part),
                _ => None,
            }
        }
        directory
    }
}
