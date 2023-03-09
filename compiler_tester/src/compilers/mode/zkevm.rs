//!
//! The compiler tester zkEVM mode.
//!

///
/// The compiler tester zkEVM mode.
///
#[derive(Debug, Default, Clone)]
pub struct Mode {}

impl Mode {
    /// The language name.
    pub const LANGUAGE: &'static str = "zkEVM";
}

impl std::fmt::Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:>8}", "zkEVM")
    }
}
