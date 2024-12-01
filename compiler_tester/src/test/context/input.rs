//!
//! Context used to process test inputs, organized in test cases.
//!

use super::case::CaseContext;

///
/// Context used to process test inputs, organized in test cases.
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InputContext<'a> {
    pub case_context: &'a CaseContext<'a>,
    pub case_name: &'a Option<String>,
    pub selector: usize,
}
