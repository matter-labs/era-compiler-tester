//!
//! The compiler toolchain to compile tests with.
//!

///
/// The compiler toolchain to compile tests with.
///
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Deserialize)]
pub enum Toolchain {
    /// The LLVM toolchain processing IRs from source level compilers
    IrLLVM,
    /// The upstream `solc` compiler.
    Solc,
    /// The forked `solc` compiler with MLIR.
    SolcLLVM,
}

impl std::str::FromStr for Toolchain {
    type Err = anyhow::Error;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        match string {
            "IR-LLVM" => Ok(Self::IrLLVM),
            "solc" => Ok(Self::Solc),
            "solc-LLVM" => Ok(Self::SolcLLVM),
            string => anyhow::bail!(
                "Unknown target `{}`. Supported targets: {:?}",
                string,
                vec![Self::IrLLVM, Self::Solc, Self::SolcLLVM]
            ),
        }
    }
}

impl std::fmt::Display for Toolchain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IrLLVM => write!(f, "IR-LLVM"),
            Self::Solc => write!(f, "solc"),
            Self::SolcLLVM => write!(f, "solc-LLVM"),
        }
    }
}
