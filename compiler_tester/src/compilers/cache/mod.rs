//!
//! The cache of compiled tests.
//!

pub mod value;

use std::collections::HashMap;
use std::hash::Hash;
use std::sync::Arc;
use std::sync::Condvar;
use std::sync::Mutex;
use std::sync::RwLock;
use std::sync::RwLockReadGuard;

use self::value::Value;

///
/// The cache of compiled tests.
///
pub struct Cache<K, V>
where
    K: Eq + Hash,
{
    /// The cache inner data structure.
    inner: RwLock<HashMap<K, Value<V>>>,
}

impl<K, V> Cache<K, V>
where
    K: Eq + Hash,
{
    ///
    /// Creates an empty cache instance.
    ///
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(HashMap::new()),
        }
    }

    ///
    /// Start computing the cache value, returns a write lock.
    ///
    pub fn start(&self, key: K) -> Option<Arc<(Mutex<()>, Condvar)>> {
        let mut inner = self.inner.write().expect("Sync");

        if inner.contains_key(&key) {
            return None;
        }

        let waiter = Value::<V>::waiter();

        inner.insert(key, Value::Waiter(waiter.clone()));

        Some(waiter)
    }

    ///
    /// Waits until value will be computed.
    ///
    pub fn wait(&self, key: &K) {
        let waiter = if let Value::Waiter(waiter) = self.read().get(key).expect("Always valid") {
            waiter.clone()
        } else {
            return;
        };
        let _guard = waiter.1.wait(waiter.0.lock().expect("Sync"));
    }

    ///
    /// Finishes computing the cache value, returns a write lock.
    ///
    /// # Panics
    ///
    /// If the value is not being computed.
    ///
    pub fn finish(&self, key: K, value: V, waiter: Arc<(Mutex<()>, Condvar)>) {
        let mut inner = self.inner.write().expect("Sync");

        assert!(
            matches!(
                inner
                    .insert(key, Value::Value(value))
                    .expect("The value is not being computed"),
                Value::Waiter(_)
            ),
            "The value is already computed"
        );

        waiter.1.notify_all();
    }

    ///
    /// Checks if value for the key is cached.
    ///
    pub fn contains(&self, key: &K) -> bool {
        self.inner.read().expect("Sync").contains_key(key)
    }

    ///
    /// Locks the cache for reading.
    ///
    pub fn read(&self) -> RwLockReadGuard<HashMap<K, Value<V>>> {
        self.inner.read().expect("Sync")
    }
}
