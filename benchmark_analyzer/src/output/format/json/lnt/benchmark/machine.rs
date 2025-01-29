//!
//! Description of the `machine` section in the JSON file generated for LNT.
//! See https://llvm.org/docs/lnt/importing_data.html
//!

///
/// Description of the `machine` section in the JSON file generated for LNT.
/// See https://llvm.org/docs/lnt/importing_data.html
///
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Machine {
    /// Machine name, for example "llvm_eravm_ir-llvm_Y+M3B3".
    pub name: String,
    /// Target name, for example "eravm" or "evm".
    pub target: era_compiler_common::Target,
    /// Code generation, for example "Y+".
    pub codegen: String,
    /// Optimization levels, for example "M3B3".
    pub optimization: String,
    /// Type of toolchain, for example, "ir-llvm".
    pub toolchain: String,
}
