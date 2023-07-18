//!
//! The thread-safe cache implementation.
//!

pub mod value;

use std::collections::HashMap;
use std::hash::Hash;
use std::sync::RwLock;

use self::value::Value;

///
/// The thread-safe cache implementation.
///
pub struct Cache<K, V>
where
    K: Eq + Hash + Clone,
    V: Clone,
{
    /// The cache inner data structure.
    inner: RwLock<HashMap<K, Value<anyhow::Result<V>>>>,
}

impl<K, V> Cache<K, V>
where
    K: Eq + Hash + Clone,
    V: Clone,
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
    /// Compute and save the cache value, if a value already started computing will do nothing.
    ///
    pub fn compute<F>(&self, key: K, f: F)
    where
        F: FnOnce() -> anyhow::Result<V>,
    {
        let waiter = Value::<V>::waiter();
        let _lock = waiter.lock().expect("Sync");
        {
            let mut inner = self.inner.write().expect("Sync");

            if inner.contains_key(&key) {
                return;
            }

            inner.insert(key.clone(), Value::Waiter(waiter.clone()));
        }

        let value = f();

        let mut inner = self.inner.write().expect("Sync");
        let entry_value = inner
            .get_mut(&key)
            .expect("The value is not being computed");

        assert!(
            matches!(entry_value, Value::Waiter(_)),
            "The value is already computed"
        );

        *entry_value = Value::Value(value);
    }

    ///
    /// Checks if value for the key is cached.
    ///
    pub fn contains(&self, key: &K) -> bool {
        self.inner.read().expect("Sync").contains_key(key)
    }

    ///
    /// Get the cloned value by the key.
    /// Will wait if the value is computing.
    ///
    /// # Panics
    ///
    /// If the value is not being computed.
    ///
    pub fn get_cloned(&self, key: &K) -> anyhow::Result<V> {
        self.wait(key);
        self.inner
            .read()
            .expect("Sync")
            .get(key)
            .expect("The value is not being computed")
            .unwrap_value()
            .as_ref()
            .map(|value| value.clone())
            .map_err(|error| anyhow::anyhow!("{}", error))
    }

    ///
    /// Waits until value will be computed if needed.
    ///
    /// # Panics
    ///
    /// If the value is not being computed.
    ///
    fn wait(&self, key: &K) {
        let waiter = if let Value::Waiter(waiter) = self
            .inner
            .read()
            .expect("Sync")
            .get(key)
            .expect("The value is not being computed")
        {
            waiter.clone()
        } else {
            return;
        };
        let _lock = waiter.lock().expect("Sync");
    }
}
