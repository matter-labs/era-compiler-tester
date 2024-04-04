//!
//! The EraVM test contract instance used for building.
//!

///
/// The EraVM test contract instance used for building.
///
#[derive(Debug, Clone)]
pub struct Instance {
    /// The contract path.
    pub path: String,
    /// The instance address.
    pub address: Option<web3::types::Address>,
    /// Whether the instance is main.
    pub is_main: bool,
    /// Whether the instance is a library.
    pub is_library: bool,
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
        is_main: bool,
        is_library: bool,
        code_hash: web3::types::U256,
    ) -> Self {
        Self {
            path,
            address,
            is_main,
            is_library,
            code_hash,
        }
    }
}
