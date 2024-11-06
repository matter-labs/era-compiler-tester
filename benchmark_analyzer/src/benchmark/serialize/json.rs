//!
//! Serializing benchmark data to JSON.
//!

use super::Benchmark;
use super::IBenchmarkSerializer;

/// Serialize the benchmark data to JSON using `serde` library.
#[derive(Default)]
pub struct Json;

impl IBenchmarkSerializer for Json {
    type Err = serde_json::error::Error;

    fn serialize_to_string(&self, benchmark: &Benchmark) -> Result<String, Self::Err> {
        serde_json::to_string(benchmark)
    }
}
