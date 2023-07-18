//!
//! The compilers cache value.
//!

use std::sync::Arc;
use std::sync::Mutex;

///
/// The compilers cache value.
///
pub enum Value<T> {
    /// The value is being computed.
    Waiter(Arc<Mutex<()>>),
    /// The value is already computed.
    Value(T),
}

impl<T> Value<T> {
    ///
    /// A shortcut waiter constructor.
    ///
    pub fn waiter() -> Arc<Mutex<()>> {
        Arc::new(Mutex::new(()))
    }

    ///
    /// Unwraps the value.
    ///
    /// # Panics
    ///
    /// If the value is computed.
    ///
    pub fn unwrap_value(&self) -> &T {
        match self {
            Self::Value(value) => value,
            _ => panic!("Value is not computed"),
        }
    }
}
