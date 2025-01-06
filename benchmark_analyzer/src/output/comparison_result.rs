//!
//! Result of comparing two benchmarks.
//!

use super::file::File;

///
/// Result of comparing two benchmarks.
///
pub enum Output {
    /// Benchmark output is a single unnamed file.
    SingleFile(String),
    /// Benchmark output is structured as a file tree, relative to some
    /// user-provided output directory.
    MultipleFiles(Vec<File>),
}
