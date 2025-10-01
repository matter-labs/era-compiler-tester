//!
//! Benchmark input source.
//!

///
/// Benchmark input source.
///
#[derive(Debug, Clone, Copy)]
pub enum Source {
    /// Tooling input source, e.g. Foundry or Hardhat.
    Tooling,
    /// Compiler tester input source.
    CompilerTester,
}

impl std::str::FromStr for Source {
    type Err = anyhow::Error;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        match string.to_lowercase().as_str() {
            "tooling" => Ok(Self::Tooling),
            "compiler-tester" => Ok(Self::CompilerTester),
            string => anyhow::bail!(
                "Unknown input source `{string}`. Supported values: {}",
                vec![Self::Tooling, Self::CompilerTester]
                    .into_iter()
                    .map(|element| element.to_string().to_lowercase())
                    .collect::<Vec<String>>()
                    .join(", ")
            ),
        }
    }
}

impl std::fmt::Display for Source {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Tooling => write!(f, "tooling"),
            Self::CompilerTester => write!(f, "compiler-tester"),
        }
    }
}
