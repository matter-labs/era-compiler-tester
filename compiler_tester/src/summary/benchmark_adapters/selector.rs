//!
//! Converts `[TestSelector]` to the representation used by the benchmark.
//!

use crate::test::selector::TestSelector;

impl From<TestSelector> for benchmark_analyzer::TestSelector {
    fn from(selector: TestSelector) -> Self {
        let TestSelector { path, case, input } = selector;
        let input = input.map(Into::into);
        benchmark_analyzer::TestSelector { path, case, input }
    }
}
