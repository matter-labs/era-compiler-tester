//!
//! Serialization of benchmark data in different output formats.
//!

pub mod csv;
pub mod json;

use crate::model::benchmark::Benchmark;

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
    fn serialize_to_string(&self, benchmark: &Benchmark) -> anyhow::Result<String, Self::Err>;
}
