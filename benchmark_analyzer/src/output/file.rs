//!
//! Represents a single benchmark output file in a set of many.
//!

use std::path::PathBuf;

///
/// Represents a single benchmark output file in a set of many.
///
pub struct File {
    /// Path to this file relative to user-provided root.
    pub path: PathBuf,
    /// File contents.
    pub contents: String,
}
