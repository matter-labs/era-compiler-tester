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
    /// The number of pattern reruns to provide more accurate benchmarks.
    pub const RUNTIME_CODE_REPEATS: usize = 16;

    ///
    /// Returns the init code.
    ///
    pub fn init_code(&self, size: usize) -> String {
        if size > 0xffff {
            panic!("The bytecode is too large");
        }
        let mut code_size = format!("60{:02x}", size);
        let mut codecopy_index = "1c";
        if size > 0xff {
            code_size = format!("61{:04x}", size);
            codecopy_index = "1d";
        }
        format!(
            "608060405234801561000f575f80fd5b50{}806100{}5f395ff3fe",
            code_size, codecopy_index
        )
    }

    ///
    /// Returns the runtime code.
    ///
    pub fn runtime_code(&self, instruction_name: &str) -> String {
        let repeats = match instruction_name {
            "RETURNDATASIZE" | "RETURNDATACOPY" | "EXTCODESIZE" | "EXTCODEHASH" | "EXTCODECOPY"
            | "CALL" | "STATICCALL" | "DELEGATECALL" | "CREATE" | "CREATE2" => 1,
            _ => Self::RUNTIME_CODE_REPEATS,
        };

        format!("{}00", self.runtime_code.repeat(repeats))
    }
}
