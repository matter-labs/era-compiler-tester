//!
//! The compilers cache value.
//!

use std::sync::Arc;
use std::sync::Condvar;
use std::sync::Mutex;

///
/// The compilers cache value.
///
pub enum Value<T> {
    /// The value is being computed.
    Waiter(Arc<(Mutex<()>, Condvar)>),
    /// The value is already computed.
    Value(T),
}

impl<T> Value<T> {
    ///
    /// A shortcut waiter constructor.
    ///
    pub fn waiter() -> Arc<(Mutex<()>, Condvar)> {
        Arc::new((Mutex::new(()), Condvar::new()))
    }

    ///
    /// Unwraps the value.
    ///
    /// # Panics
    ///
    /// If the value is being waited for.
    ///
    pub fn unwrap_value(&self) -> &T {
        match self {
            Self::Value(value) => value,
            _ => panic!("Not a value"),
        }
    }
}
