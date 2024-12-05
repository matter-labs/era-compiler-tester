//!
//! Native JSON format that corresponds to the inner benchmark analyzer data model.
//!

use crate::model::benchmark::Benchmark;
use crate::format::IBenchmarkSerializer;

/// Serialize the benchmark data to JSON using `serde` library.
#[derive(Default)]
pub struct Json;

impl IBenchmarkSerializer for Json {
    type Err = serde_json::error::Error;

    fn serialize_to_string(&self, benchmark: &Benchmark) -> Result<String, Self::Err> {
        serde_json::to_string_pretty(benchmark)
    }
}
