//!
//! Benchmark-analyzer output.
//!

pub mod comparison_result;
pub mod file;
pub mod format;

use comparison_result::Output;

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
    fn serialize_to_string(&self, benchmark: &Benchmark) -> anyhow::Result<Output, Self::Err>;
}
