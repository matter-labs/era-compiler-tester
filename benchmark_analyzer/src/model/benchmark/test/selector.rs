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
    /// Project name.
    pub project: String,
    /// Name of the case, if any. `None` means nameless case.
    pub case: Option<String>,
    /// Identifier of the specific input.
    pub input: Option<Input>,
}

impl Selector {
    ///
    /// Returns the identifier for XLSX reports.
    ///
    pub fn xlsx_identifier(&self) -> String {
        let Self {
            project,
            case,
            input,
        } = self;
        let mut identifier = project.clone();
        if let Some(case) = case {
            identifier.push('/');
            identifier.push_str(case.as_str());
        }
        if let Some(Input::Runtime { name, .. }) = input {
            identifier.push('.');
            identifier.push_str(name.as_str());
        }
        identifier
    }
}

impl std::fmt::Display for Selector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self {
            project,
            case,
            input,
            ..
        } = self;
        f.write_fmt(format_args!("{project}"))?;
        if let Some(case) = case {
            f.write_fmt(format_args!("::{case}"))?;
        }
        if let Some(input) = input {
            f.write_fmt(format_args!("[{input}]"))?;
        }
        Ok(())
    }
}
