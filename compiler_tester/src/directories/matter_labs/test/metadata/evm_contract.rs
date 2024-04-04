//!
//! The Matter Labs compiler test metadata EVM contract.
//!

use serde::Deserialize;

///
/// The Matter Labs compiler test metadata EVM contract.
///
#[derive(Debug, Clone, Deserialize)]
pub struct EVMContract {
    /// The runtime code.
    runtime_code: String,
}

impl EVMContract {
    ///
    /// Returns the init code.
    ///
    pub fn init_code(&self) -> String {
        "608060405234801561000f575f80fd5b5060c08061001c5f395ff3fe".to_owned()
    }

    ///
    /// Returns the runtime code.
    ///
    pub fn runtime_code(&self) -> String {
        let mut runtime_code = self.runtime_code.repeat(16 /* TODO */).to_owned();
        runtime_code.push_str("00");
        runtime_code
    }
}
