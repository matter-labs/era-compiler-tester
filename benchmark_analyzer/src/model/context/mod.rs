//!
//! A context for benchmarking, passed by compiler-tester.
//!

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

///
/// Checks that the context is well-formed.
///
pub fn validate_context(context: &Context) -> anyhow::Result<()> {
    let Context {
        machine,
        toolchain,
        target: _,
        zksolc_version: _,
        llvm_version: _,
    } = context;

    if machine.is_empty() {
        anyhow::bail!("The `machine` field in the benchmark context is empty")
    }
    if toolchain.is_empty() {
        anyhow::bail!("The `toolchain` field in the benchmark context is empty")
    }
    Ok(())
}
