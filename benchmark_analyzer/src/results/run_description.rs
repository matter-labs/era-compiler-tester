//!
//! An entry in benchmark comparison results table.
//!

use crate::model::benchmark::test::codegen::versioned::executable::metadata::Metadata as ExecutableMetadata;
use crate::model::benchmark::test::codegen::versioned::Mode;
use crate::model::benchmark::test::codegen::Version;
use crate::model::benchmark::test::metadata::Metadata as TestMetadata;
use crate::model::benchmark::test::Codegen;
use crate::Run;

///
/// An entry in benchmark comparison results table.
///
#[derive(Clone, Debug)]
pub struct RunDescription<'a> {
    /// Metadata of a test. It is common for test runs with different language versions, or compilation options
    pub test_metadata: &'a TestMetadata,
    /// Language version, if applicable
    pub version: &'a Version,
    /// Language version, if applicable
    pub codegen: &'a Codegen,
    /// Compiler options
    pub mode: &'a Mode,
    /// Metadata associated with the compiled binary
    pub executable_metadata: &'a ExecutableMetadata,
    /// Measurements
    pub run: &'a Run,
}

impl<'a> std::fmt::Display for RunDescription<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let RunDescription {
            test_metadata: TestMetadata { selector, .. },
            version,
            codegen,
            mode,
            ..
        } = self;

        f.write_fmt(format_args!("{codegen}{mode} {version} {selector}"))
    }
}
