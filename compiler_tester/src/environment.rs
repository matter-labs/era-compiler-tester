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
    /// The new fast EraVM.
    FastVM,
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
            "FastVM" => Ok(Self::FastVM),
            "EVMInterpreter" => Ok(Self::EVMInterpreter),
            "REVM" => Ok(Self::REVM),
            string => anyhow::bail!(
                "Unknown environment `{}`. Supported environments: {:?}",
                string,
                vec![Self::ZkEVM, Self::FastVM, Self::EVMInterpreter, Self::REVM]
            ),
        }
    }
}

impl From<Environment> for era_compiler_common::Target {
    fn from(environment: Environment) -> Self {
        match environment {
            Environment::ZkEVM => era_compiler_common::Target::EraVM,
            Environment::FastVM => era_compiler_common::Target::EraVM,
            Environment::EVMInterpreter => era_compiler_common::Target::EVM,
            Environment::REVM => era_compiler_common::Target::EVM,
        }
    }
}

impl std::fmt::Display for Environment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ZkEVM => write!(f, "zk_evm"),
            Self::FastVM => write!(f, "FastVM"),
            Self::EVMInterpreter => write!(f, "EVMInterpreter"),
            Self::REVM => write!(f, "REVM"),
        }
    }
}
