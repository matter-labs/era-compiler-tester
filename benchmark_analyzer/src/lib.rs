//!
//! The benchmark analyzer library.
//!

pub(crate) mod benchmark;

pub use self::benchmark::group::element::Element as BenchmarkElement;
pub use self::benchmark::group::Group as BenchmarkGroup;
pub use self::benchmark::serialize::csv::Csv as CsvSerializer;
pub use self::benchmark::serialize::json::Json as JsonSerializer;
pub use self::benchmark::Benchmark;
///
/// The all elements group name.
///
pub const BENCHMARK_ALL_GROUP_NAME: &str = "All";
