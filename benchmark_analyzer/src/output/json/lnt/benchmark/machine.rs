//!
//! LNT machine description.
//!

///
/// Description of the `machine` section in the JSON file generated for LNT.
/// See https://llvm.org/docs/lnt/importing_data.html
///
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Machine {
    /// Machine name, such as `llvm_eravm_ir-llvm_Y+M3B3`.
    pub name: String,
    /// Target name, such as `eravm` or `evm`.
    pub target: era_compiler_common::Target,
    /// Code generation, such as `Y+`.
    pub codegen: String,
    /// Optimization levels, such as `M3B3`.
    pub optimization: String,
    /// Type of toolchain, such as `ir-llvm`.
    pub toolchain: String,
}
