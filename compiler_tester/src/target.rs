//!
//! The compiler tester target to run tests on.
//!

///
/// The compiler tester target to run tests on.
///
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Deserialize)]
pub enum Target {
    /// The EraVM target.
    EraVM,
    /// The native EVM target.
    EVM,
    /// The EVM interpreter running on top of EraVM.
    EVMInterpreter,
    // The Rust Ethereum Virtual Machine target.
    REVM,
}

impl std::str::FromStr for Target {
    type Err = anyhow::Error;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        match string {
            "EraVM" => Ok(Self::EraVM),
            "EVM" => Ok(Self::EVM),
            "EVMInterpreter" => Ok(Self::EVMInterpreter),
            "REVM" => Ok(Self::REVM),
            string => Err(anyhow::anyhow!(
                "Unknown target `{}`. Supported targets: {:?}",
                string,
                vec![Self::EraVM, Self::EVM, Self::EVMInterpreter]
            )),
        }
    }
}

impl std::fmt::Display for Target {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Target::EraVM => write!(f, "EraVM"),
            Target::EVM => write!(f, "EVM"),
            Target::REVM => write!(f, "REVM"),
            Target::EVMInterpreter => write!(f, "EVMInterpreter"),
        }
    }
}
