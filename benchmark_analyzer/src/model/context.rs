//!
//! A context for benchmarking, passed by compiler-tester.
//!

use std::path::PathBuf;

///
/// A context for benchmarking, passed by compiler-tester.
///
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Context {
    /// Unique identifier of the machine.
    pub machine: String,
    /// Target, for example "eravm" or "evm".
    pub target: era_compiler_common::Target,
    /// Type of `toolchain`, for example `ir-llvm`
    pub toolchain: String,
    /// Version of the `zksolc` compiler.
    pub zksolc_version: String,
    /// Version of the LLVM backend.
    pub llvm_version: String,
}

impl Context {
    ///
    /// Reads the benchmarking context from a JSON file and validates its correctness.
    /// Benchmarking context provides additional information about benchmarking that
    /// will be used to generate a report.
    ///
    /// # Errors
    ///
    /// 1. File cannot be read.
    /// 2. Deserialization from JSON file failed.
    /// 3. Context validation failed.
    ///
    pub fn try_from_path(path: PathBuf) -> anyhow::Result<Self> {
        let contents = std::fs::read_to_string(path)?;
        let context: Self = serde_json::de::from_str(&contents)?;
        context.validate()?;
        Ok(context)
    }

    ///
    /// Checks that the context is well-formed.
    ///
    pub fn validate(&self) -> anyhow::Result<()> {
        let Context {
            machine,
            toolchain,
            target: _,
            zksolc_version: _,
            llvm_version: _,
        } = self;

        if machine.is_empty() {
            anyhow::bail!("The `machine` field in the benchmark context is empty")
        }
        if toolchain.is_empty() {
            anyhow::bail!("The `toolchain` field in the benchmark context is empty")
        }
        Ok(())
    }
}
