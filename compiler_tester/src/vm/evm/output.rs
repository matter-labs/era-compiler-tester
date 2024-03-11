//!
//! The EVM output.
//!

///
/// The EVM output.
///
pub struct Output {
    /// The return data.
    pub return_data: Vec<u8>,
    /// The exception flag.
    pub exception: bool,
    /// The emitted logs.
    pub logs: Vec<evm::Log>,
}

impl Output {
    ///
    /// A shortcut constructor.
    ///
    pub fn new(return_data: Vec<u8>, exception: bool, logs: Vec<evm::Log>) -> Self {
        Self {
            return_data,
            exception,
            logs,
        }
    }
}
