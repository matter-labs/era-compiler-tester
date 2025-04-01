//!
//! The compiler tester EraVM mode.
//!

use crate::compilers::mode::imode::IMode;

///
/// The compiler tester EraVM mode.
///
#[derive(Debug, Default, Clone, Eq, PartialEq)]
pub struct Mode {}

impl std::fmt::Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "")
    }
}
impl IMode for Mode {
    fn optimizations(&self) -> Option<String> {
        None
    }

    fn codegen(&self) -> Option<String> {
        None
    }

    fn version(&self) -> Option<String> {
        None
    }
}
