//!
//! The compiler tester workflows.
//!

use std::str::FromStr;

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

impl FromStr for Workflow {
    type Err = anyhow::Error;

    fn from_str(day: &str) -> Result<Self, Self::Err> {
        match day {
            "build" => Ok(Workflow::BuildOnly),
            "run" => Ok(Workflow::BuildAndRun),
            string => anyhow::bail!(
                "Unknown workflow `{}`. Supported workflows: {:?}",
                string,
                vec![Self::BuildOnly, Self::BuildAndRun]
            ),
        }
    }
}
