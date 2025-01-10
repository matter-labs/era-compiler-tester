//!
//! Converts `[TestDescription]` to the representation used by the benchmark.
//!

use crate::test::description::TestDescription;

impl From<TestDescription> for benchmark_analyzer::ExecutableMetadata {
    fn from(_: TestDescription) -> Self {
        benchmark_analyzer::ExecutableMetadata {}
    }
}
