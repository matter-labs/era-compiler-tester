//!
//! Description of the benchmark run in a JSON file passed to LNT.
//!

use chrono::DateTime;
use chrono::Utc;

///
/// Run order in the LNT report. Equals to DateTime<Utc> for now.
///
type LLVMProjectRevision = DateTime<Utc>;

///
/// Description of the benchmark run in a JSON file passed to LNT.
///
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct RunDescription {
    /// LNT run order.
    pub llvm_project_revision: LLVMProjectRevision,
    /// Time when benchmark run was started.
    pub start_time: DateTime<Utc>,
    /// Time when benchmark run was finished.
    pub end_time: DateTime<Utc>,
    /// Version of the `zksolc` compiler.
    pub zksolc_version: String,
    /// Version of the LLVM backend.
    pub llvm_version: String,
}
