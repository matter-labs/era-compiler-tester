//!
//! Converts `[TestSelector]` to the representation used by the benchmark.
//!

use crate::test::selector::TestSelector;

use super::input::convert_input;

///
/// Converts `[TestSelector]` to the representation used by the benchmark.
///
pub fn convert_selector(selector: TestSelector) -> benchmark_analyzer::TestSelector {
    let TestSelector { path, case, input } = selector;
    let input = input.map(convert_input);
    benchmark_analyzer::TestSelector { path, case, input }
}
