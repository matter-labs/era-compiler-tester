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
    /// Type of `toolchain`, such as `ir-llvm`
    pub toolchain: String,
    /// Version of the compiler.
    pub compiler_version: String,
    /// Version of the LLVM backend.
    pub llvm_version: String,
    /// Target, such as `evm` or `eravm`.
    pub target: era_compiler_common::Target,

    /// Global codegen setting.
    pub codegen: Option<String>,
    /// Global optimization settings.
    pub optimization: Option<String>,
}

impl Context {
    ///
    /// Checks that the context is well-formed.
    ///
    pub fn validate(&self) -> anyhow::Result<()> {
        let Context {
            machine, toolchain, ..
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

impl TryFrom<PathBuf> for Context {
    type Error = anyhow::Error;

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
    fn try_from(path: PathBuf) -> anyhow::Result<Self> {
        let text = std::fs::read_to_string(path.as_path())
            .map_err(|error| anyhow::anyhow!("Benchmark context file {path:?} reading: {error}"))?;
        let context: Self = serde_json::from_str(text.as_str())
            .map_err(|error| anyhow::anyhow!("Benchmark context file {path:?} parsing: {error}"))?;
        context.validate()?;
        Ok(context)
    }
}
