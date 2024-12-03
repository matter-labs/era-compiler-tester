//!
//! Converts `[TestSelector]` to the representation used by the benchmark.
//!

use crate::test::selector::TestSelector;

///
/// Converts `[TestSelector]` to the representation used by the benchmark.
///
pub fn convert_selector(selector: TestSelector) -> benchmark_analyzer::TestSelector {
    let TestSelector { path, case, input } = selector;
    let input = input.map(Into::into);
    benchmark_analyzer::TestSelector { path, case, input }
}
