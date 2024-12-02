//!
//! Converts `[TestDescription]` to the representation used by the benchmark.
//!

use crate::test::description::TestDescription;

use super::selector::convert_selector;

///
/// Converts `[TestSelector]` to the representation used by the benchmark.
///

pub fn convert_description(
    description: &TestDescription,
    default_group: &str,
) -> benchmark_analyzer::Metadata {
    let TestDescription {
        group,
        mode,
        selector,
    } = description.clone();
    let selector = convert_selector(selector);
    let mode = mode.map(|m| m.to_string());
    let group = group.unwrap_or(default_group.to_string());
    benchmark_analyzer::Metadata {
        selector,
        mode,
        group,
    }
}
