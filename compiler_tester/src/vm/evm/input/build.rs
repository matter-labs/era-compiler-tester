//!
//! The EVM contract build.
//!

///
/// The EVM contract build.
///
#[derive(Debug, Clone)]
pub struct Build {
    /// The contract deploy build.
    pub deploy_build: Vec<u8>,
    /// The contract runtime build.
    pub runtime_build: Vec<u8>,
}

impl Build {
    ///
    /// A shortcut constructor.
    ///
    pub fn new(deploy_build: Vec<u8>, runtime_build: Vec<u8>) -> Self {
        Self {
            deploy_build,
            runtime_build,
        }
    }
}
