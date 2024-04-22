//!
//! Compiler tester workflows
//!

///
/// Describes sets of actions that compiler tester is able to perform.
///
#[derive(Debug)]
pub enum Workflow {
    /// Only build tests but not execute them.
    BuildOnly,
    /// Build and execute tests.
    BuildAndRun,
}
use std::str::FromStr;

// any error type implementing Display is acceptable.
type ParseError = &'static str;

impl FromStr for Workflow {
    type Err = ParseError;
    fn from_str(day: &str) -> Result<Self, Self::Err> {
        match day {
            "build" => Ok(Workflow::BuildOnly),
            "run" => Ok(Workflow::BuildAndRun),
            _ => Err("Could not parse workflow. Supported workflows: build, run."),
        }
    }
}
