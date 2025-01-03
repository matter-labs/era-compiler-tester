//!
//! A context for benchmarking, passed by compiler-tester.
//!

///
/// A context for benchmarking, passed by compiler-tester.
///
#[derive(Clone, Debug, Default, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Context {
    /// Unique identifier of the machine.
    pub machine: String,
    /// Target, for example as "eravm" or "evm".
    pub target: String,
    /// Type of `solc`, for example `zksync`
    pub solc_type: String,
}

///
/// Checks that the context is well-formed.
///
pub fn validate_context(context: &Context) -> anyhow::Result<()> {
    let Context {
        machine,
        target,
        solc_type,
    } = context;

    if machine.is_empty() {
        anyhow::bail!("The `machine` field in the benchmark context is empty")
    }
    if target.is_empty() {
        anyhow::bail!("The `target` field in the benchmark context is empty")
    }
    if solc_type.is_empty() {
        anyhow::bail!("The `solc_type` field in the benchmark context is empty")
    }
    Ok(())
}
