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
    /// Machine name, for example "LNT-AArch64-A53-O3__clang_DEV__aarch64".
    pub name: String,
    /// Target name, for example "eravm" or "solc".
    pub target: era_compiler_common::Target,
    /// Optimizations level, for example "+M3B3".
    pub optimizations: String,
    /// Type of solc, for example, "zksync".
    pub toolchain: String,
}
