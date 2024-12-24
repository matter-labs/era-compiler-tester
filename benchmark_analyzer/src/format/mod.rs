//!
//! Serialization of benchmark data in different output formats.
//!

pub mod csv;
pub mod json;

use std::path::PathBuf;

use crate::model::benchmark::Benchmark;

///
/// Benchmark comparison output.
///
pub enum Output {
    /// Benchmark output is a single anonymous file.
    SingleFile(String),
    /// Benchmark output is structured as a file tree.
    MultipleFiles(Vec<File>),
}

///
/// Represents a single benchmark output file in a set of many.
///
pub struct File {
    /// Path to this file relative to user-provided root.
    pub path: PathBuf,
    /// File contents.
    pub contents: String,
}

///
/// Serialization format for benchmark data.
///
pub trait IBenchmarkSerializer {
    ///
    /// Type of serialization error.
    ///
    type Err: std::error::Error;

    ///
    /// Serialize benchmark data in the selected format.
    ///
    fn serialize_to_string(&self, benchmark: &Benchmark) -> anyhow::Result<Output, Self::Err>;
}
