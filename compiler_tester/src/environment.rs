//!
//! The tester environment to run tests on.
//!

///
/// The tester environment to run tests on.
///
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Deserialize)]
pub enum Environment {
    /// The old `zk_evm` EraVM.
    ZkEVM,
    /// The EraVM-based EVM interpreter.
    EVMInterpreter,
    /// The REVM implementation.
    REVM,
}

impl std::str::FromStr for Environment {
    type Err = anyhow::Error;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        match string {
            "zk_evm" => Ok(Self::ZkEVM),
            "EVMInterpreter" => Ok(Self::EVMInterpreter),
            "REVM" => Ok(Self::REVM),
            string => anyhow::bail!(
                "Unknown environment `{}`. Supported environments: {:?}",
                string,
                vec![Self::ZkEVM, Self::EVMInterpreter, Self::REVM]
                    .into_iter()
                    .map(|element| element.to_string())
                    .collect::<Vec<String>>()
                    .join(", ")
            ),
        }
    }
}

impl From<Environment> for benchmark_analyzer::Target {
    fn from(environment: Environment) -> Self {
        match environment {
            Environment::ZkEVM => Self::EraVM,
            Environment::EVMInterpreter => Self::EVM,
            Environment::REVM => Self::EVM,
        }
    }
}

impl std::fmt::Display for Environment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ZkEVM => write!(f, "zk_evm"),
            Self::EVMInterpreter => write!(f, "EVMInterpreter"),
            Self::REVM => write!(f, "REVM"),
        }
    }
}
