//!
//! Information associated with the benchmark run.
//!

use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;

use crate::model::context::Context;

///
/// Information associated with the benchmark run.
///
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Metadata {
    /// Start of the benchmark run.
    pub start: DateTime<Utc>,
    /// End of the benchmark run.
    pub end: DateTime<Utc>,
    /// Context of benchmarking, passed from compiler tester.
    #[serde(skip)]
    pub context: Option<Context>,
}
