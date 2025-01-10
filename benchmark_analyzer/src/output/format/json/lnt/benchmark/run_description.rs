//!
//! Description of the benchmark run in a JSON file passed to LNT.
//!

use chrono::DateTime;
use chrono::Utc;

///
/// Description of the benchmark run in a JSON file passed to LNT.
///
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct RunDescription {
    /// Time when benchmark run was started.
    pub start_time: DateTime<Utc>,
    /// Time when benchmark run was finished.
    pub end_time: DateTime<Utc>,
}
