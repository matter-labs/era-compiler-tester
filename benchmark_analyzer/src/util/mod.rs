//!
//! Utility functions.
//!

pub mod btreemap;

///
/// Check if a value is zero.
/// This is a helper function for serialization.
///
pub fn is_zero<T: PartialEq + From<u8>>(value: &T) -> bool {
    *value == T::from(0)
}
