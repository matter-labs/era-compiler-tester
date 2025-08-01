//!
//! Description of a single measurement in a JSON file passed to LNT.
//!

use crate::model::benchmark::test::toolchain::codegen::versioned::executable::run::Run;

///
/// Description of a single measurement in a JSON file passed to LNT.
///
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct TestDescription {
    /// A unique identifier of the test, incorporating language version, optimization levels and so on.
    /// See [crate::output::format::json::lnt::test_name].
    pub name: String,
    /// Measurements: gas, ergs, cycles, and size for contract deploys.
    #[serde(flatten)]
    pub measurements: Run,
}
