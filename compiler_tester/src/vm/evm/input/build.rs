//!
//! The EVM contract build.
//!

///
/// The EVM contract build.
///
#[derive(Debug, Clone)]
pub struct Build {
    /// The contract deploy build.
    pub deploy_build: era_compiler_llvm_context::EVMBuild,
    /// The contract runtime build.
    pub runtime_build: era_compiler_llvm_context::EVMBuild,
}

impl Build {
    ///
    /// A shortcut constructor.
    ///
    pub fn new(
        deploy_build: era_compiler_llvm_context::EVMBuild,
        runtime_build: era_compiler_llvm_context::EVMBuild,
    ) -> Self {
        Self {
            deploy_build,
            runtime_build,
        }
    }
}
