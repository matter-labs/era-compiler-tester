//!
//! The test contract instance used for building.
//!

///
/// The test contract instance used for building.
///
#[derive(Debug, Clone)]
pub struct Instance {
    /// The contract path.
    pub path: String,
    /// The instance address.
    pub address: Option<web3::types::Address>,
    /// The contract bytecode hash.
    pub code_hash: web3::types::U256,
}

impl Instance {
    ///
    /// A shortcut constructor.
    ///
    pub fn new(
        path: String,
        address: Option<web3::types::Address>,
        code_hash: web3::types::U256,
    ) -> Self {
        Self {
            path,
            address,
            code_hash,
        }
    }
}
