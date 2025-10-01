//!
//! Utility functions
//!

use std::collections::BTreeMap;

/// Intersects two `BTreeMap` instances and merges their entries using a
/// specified merger function.
///
/// # Arguments
///
/// * `map1` - The first `BTreeMap` containing keys of type `K` and values of
///   type `V1`.
/// * `map2` - The second `BTreeMap` containing keys of type `K` and values of
///   type `V2`. This map is modified during the intersection.
/// * `merger` - A closure that takes a key of type `K`, and a value from each
///   map (`V1` and `V2`), and returns a merged result of type `R`.
///
/// # Returns
///
/// An iterator that yields merged results of type `R` for each intersecting key
/// from the maps.
///
/// # Example
///
/// ```rust
/// use benchmark_converter::util::btreemap::intersect_map;
///
/// let first = [(1, 1), (2, 2), (3, 3)];
/// let second = [(1, 10), (3, 30)];
/// let expected: Vec<_> = [111, 333].into();
/// assert_eq!(
/// intersect_map(first.into(), second.into(), |k, v1, v2| 100 * k + v1 + v2)
/// .collect::<Vec<_>>(),
/// expected
/// )
/// ```
pub fn intersect_map<K, V1, V2, R>(
    map1: BTreeMap<K, V1>,
    mut map2: BTreeMap<K, V2>,
    merger: impl Fn(K, V1, V2) -> R + 'static,
) -> impl Iterator<Item = R>
where
    K: Ord,
{
    map1.into_iter().filter_map(move |(key, value1)| {
        map2.remove(&key).map(|value2| merger(key, value1, value2))
    })
}

/// Perform a cross join on two `BTreeMap` instances, applying a
/// selector function to each pair of keys. If the selector function returns an
/// `Option::Some`, it includes the transformed key along with cloned values
/// from both maps into the result vector.
///
/// # Arguments
///
/// * `map1` - A reference to the first `BTreeMap` with key type `K` and value
///   type `V1`.
/// * `map2` - A reference to the second `BTreeMap` with key type `K` and value
///   type `V2`.
/// * `selector` - A closure or function that takes two keys (one from each map)
///   and returns an `Option<N>`, where `N` is the type of the transformed key
///   to be included in the result if the option is `Some`.
///
/// # Returns
///
/// A vector containing tuples of the transformed key type `N`, and values from
/// both maps (`V1` and `V2`), corresponding to each pair of matched keys for
/// which the selector function has returned `Some`.
///
/// # Type Parameters
///
/// * `K` - The type of key used in both input maps, which must implement `Ord`.
/// * `N` - The type for transformed key pairs.
/// * `V1` - The type of value in the first map, which must implement `Clone`.
/// * `V2` - The type of value in the second map, which must implement `Clone`.
///
/// # Example
///
/// ```rust
/// use std::collections::BTreeMap;
///
/// use benchmark_converter::util::btreemap::cross_join_filter_map;
///
/// // Assume we have two BTreeMaps.
/// let map1: BTreeMap<_, _> = [(1, "a"), (2, "b")].iter().cloned().collect();
/// let map2: BTreeMap<_, _> = [(1, "x"), (2, "y")].iter().cloned().collect();
///
/// // Define a selector function that combines the keys.
/// let selector = |k1: &i32, k2: &i32| if k1 == k2 { Some(k1 + k2) } else { None };
///
/// // Execute the cross join with filtering using the selector.
/// let result = cross_join_filter_map(&map1, &map2, selector);
///
/// // Result now contains: [(2, "a", "x"), (4, "b", "y")]
/// assert_eq!(result, vec![(2, "a", "x"), (4, "b", "y")]);
/// ```
pub fn cross_join_filter_map<K, N, V1, V2>(
    map1: &BTreeMap<K, V1>,
    map2: &BTreeMap<K, V2>,
    selector: impl Fn(&K, &K) -> Option<N>,
) -> Vec<(N, V1, V2)>
where
    K: Ord,
    V1: Clone,
    V2: Clone,
{
    let mut result: Vec<(N, V1, V2)> = Vec::new();

    for (key1, value1) in map1 {
        for (key2, value2) in map2 {
            if let Some(new_key) = selector(key1, key2) {
                result.push((new_key, value1.clone(), value2.clone()));
            }
        }
    }

    result
}

/// Returns an iterator over
/// the elements that are common to both `map1` and `map2`.
///
/// # Arguments
///
/// * `map1` - A BTreeMap where the keys are compared.
/// * `map2` - A mutable BTreeMap from which matching keys are removed and their values paired with those from `map1`.
///
/// # Returns
///
/// An iterator over tuples `(K, V1, V2)` where:
/// * `K` is the common key.
/// * `V1` is the associated value from `map1`.
/// * `V2` is the associated value from `map2`.
///
/// The iterator only includes keys that are present in both maps.
///
/// # Example
///
/// ```rust
/// use benchmark_converter::util::btreemap::intersect_keys;
///
/// let first = [(1, "1"), (2, "2"), (3, "3")];
/// let second = [(1, "11"), (3, "33")];
/// let expected: Vec<_> = [(1, "1", "11"), (3, "3", "33")].into();
/// assert_eq!(
/// intersect_keys(first.into(), second.into()).collect::<Vec<_>>(),
/// expected
/// )
/// ```
pub fn intersect_keys<K, V1, V2>(
    map1: BTreeMap<K, V1>,
    mut map2: BTreeMap<K, V2>,
) -> impl Iterator<Item = (K, V1, V2)>
where
    K: Ord,
{
    map1.into_iter()
        .filter_map(move |(key, value1)| map2.remove(&key).map(|value2| (key, value1, value2)))
}
