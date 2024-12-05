//!
//! Information associated with the benchmark run.
//!

use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;

/// Version of the benchmark format.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub enum BenchmarkVersion {
    #[default]
    /// Flat format, a map from key (Identifier + mode) to measurements.
    V1,
    /// New format with metadata.
    V2,
}

///
/// Information associated with the benchmark run.
///
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Metadata {
    /// Version for the benchmark report.
    pub version: BenchmarkVersion,
    /// Start of the benchmark run.
    pub start: DateTime<Utc>,
    /// End of the benchmark run.
    pub end: DateTime<Utc>,
}
