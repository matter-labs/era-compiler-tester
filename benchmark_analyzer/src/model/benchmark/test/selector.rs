//!
//! Test selector, unambiously locating a test suite, or a specific input.
//!

use serde::Deserialize;
use serde::Serialize;

use crate::model::benchmark::test::input::Input;

///
/// Test selector, unambiously locating a test suite, case, or input.
///
#[derive(Debug, Clone, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct Selector {
    /// Path to the file containing test.
    pub path: String,
    /// Name of the case, if any. `None` means nameless case.
    pub case: Option<String>,
    /// Identifier of the specific input.
    pub input: Option<Input>,
}

impl std::fmt::Display for Selector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self {
            path: filename,
            case: case_name,
            input,
        } = self;
        f.write_fmt(format_args!("{filename}"))?;
        if let Some(case_name) = case_name {
            f.write_fmt(format_args!("::{case_name}"))?;
        }
        if let Some(input) = input {
            f.write_fmt(format_args!("[{input}]"))?;
        }
        Ok(())
    }
}
