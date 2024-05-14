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
    pub fn init_code(&self, size: usize) -> String {
        format!("608060405234801561000f575f80fd5b50{}8061001c5f395ff3fe", if size <= 0xff {
            format!("60{:02x}", size)
        } else if size <= 0xffff {
            format!("61{:04x}", size)
        } else {
            panic!("The bytecode is too large");
        })
    }

    ///
    /// Returns the runtime code.
    ///
    pub fn runtime_code(&self) -> String {
        format!("{}00", self.runtime_code)
    }
}
