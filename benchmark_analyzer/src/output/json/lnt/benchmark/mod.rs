//! Root benchmark structure describing a single JSON file passed to LNT.
//! One such file is generated for every machine configuration.
//! See https://llvm.org/docs/lnt/importing_data.html

pub mod machine;
pub mod run_description;
pub mod test_description;

use machine::Machine;
use run_description::RunDescription;
use serde::{Deserialize, Serialize};
use test_description::TestDescription;

///
/// Root benchmark structure describing a single JSON file passed to LNT.
/// One such file is generated for every machine configuration.
/// See https://llvm.org/docs/lnt/importing_data.html
///
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct LntBenchmark {
    /// Benchmark format version
    pub format_version: LntReportFormatVersion,
    /// Machine description is used as a group identifier
    pub machine: Machine,
    /// Describes the runtime benchmark characteristics, for example, when it has started and when it has ended
    pub run: RunDescription,
    /// Tests grouped in this benchmark.
    pub tests: Vec<TestDescription>,
}

///
/// Version of the LNT report.
///
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum LntReportFormatVersion {
    /// Old deprecated LNT report format.
    #[serde(rename = "1")]
    V1,
    /// New LNT report format.
    #[serde(rename = "2")]
    V2,
}
