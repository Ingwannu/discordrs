use std::cmp::Ordering;
use std::collections::HashMap;
use std::hash::Hash;

/// A `HashMap` wrapper with utility methods, paralleling discord.js's `Collection<K, V>`.
///
/// Provides functional iteration helpers like `find`, `filter`, `partition`,
/// `sweep`, `some`, `every`, and more — idiomatic Rust equivalents of
/// the JavaScript Collection API.
#[derive(Clone, Debug)]
pub struct Collection<K, V>(HashMap<K, V>);

impl<K, V> Collection<K, V>
where
    K: Eq + Hash + Clone,
    V: Clone,
{
    /// Creates an empty collection.
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    /// Creates an empty collection with the specified capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self(HashMap::with_capacity(capacity))
    }

    /// Inserts a key-value pair, returning the old value if present.
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        self.0.insert(key, value)
    }

    /// Returns a reference to the value for the given key.
    pub fn get(&self, key: &K) -> Option<&V> {
        self.0.get(key)
    }

    /// Returns a mutable reference to the value for the given key.
    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        self.0.get_mut(key)
    }

    /// Removes a key-value pair, returning the value if present.
    pub fn remove(&mut self, key: &K) -> Option<V> {
        self.0.remove(key)
    }

    /// Returns `true` if the collection contains the given key.
    pub fn contains_key(&self, key: &K) -> bool {
        self.0.contains_key(key)
    }

    /// Returns the number of elements.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns `true` if the collection is empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Removes all elements.
    pub fn clear(&mut self) {
        self.0.clear();
    }

    /// Returns an iterator over key-value pairs.
    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.0.iter()
    }

    /// Returns an iterator over keys.
    pub fn keys(&self) -> impl Iterator<Item = &K> {
        self.0.keys()
    }

    /// Returns an iterator over values.
    pub fn values(&self) -> impl Iterator<Item = &V> {
        self.0.values()
    }

    // --- discord.js-style utility methods ---

    /// Finds the first value matching the predicate.
    ///
    /// Parallels `Collection.find(v => condition)` in discord.js.
    pub fn find(&self, predicate: impl Fn(&K, &V) -> bool) -> Option<&V> {
        self.0.iter().find(|(k, v)| predicate(k, v)).map(|(_, v)| v)
    }

    /// Returns a new Collection containing only entries matching the predicate.
    ///
    /// Parallels `Collection.filter((v, k) => condition)`.
    pub fn filter(&self, predicate: impl Fn(&K, &V) -> bool) -> Self {
        let filtered: HashMap<K, V> = self
            .0
            .iter()
            .filter(|(k, v)| predicate(k, v))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        Self(filtered)
    }

    /// Partitions the collection into two: (matching, non-matching).
    ///
    /// Parallels `Collection.partition((v, k) => condition)`.
    pub fn partition(&self, predicate: impl Fn(&K, &V) -> bool) -> (Self, Self) {
        let mut matching = HashMap::new();
        let mut non_matching = HashMap::new();
        for (k, v) in &self.0 {
            if predicate(k, v) {
                matching.insert(k.clone(), v.clone());
            } else {
                non_matching.insert(k.clone(), v.clone());
            }
        }
        (Self(matching), Self(non_matching))
    }

    /// Maps each entry to a new value, returning a Vec.
    ///
    /// Parallels `Collection.map((v, k) => result)`.
    pub fn map<O>(&self, f: impl Fn(&K, &V) -> O) -> Vec<O> {
        self.0.iter().map(|(k, v)| f(k, v)).collect()
    }

    /// Maps each entry and flattens the results.
    ///
    /// Parallels `Collection.flatMap((v, k) => [...results])`.
    pub fn flat_map<O>(&self, f: impl Fn(&K, &V) -> Vec<O>) -> Vec<O> {
        self.0.iter().flat_map(|(k, v)| f(k, v)).collect()
    }

    /// Maps entries that match the predicate, returning a Vec.
    pub fn filter_map<O>(&self, f: impl Fn(&K, &V) -> Option<O>) -> Vec<O> {
        self.0.iter().filter_map(|(k, v)| f(k, v)).collect()
    }

    /// Returns `true` if any entry matches the predicate.
    ///
    /// Parallels `Collection.some((v, k) => condition)`.
    pub fn some(&self, predicate: impl Fn(&K, &V) -> bool) -> bool {
        self.0.iter().any(|(k, v)| predicate(k, v))
    }

    /// Returns `true` if all entries match the predicate.
    ///
    /// Parallels `Collection.every((v, k) => condition)`.
    pub fn every(&self, predicate: impl Fn(&K, &V) -> bool) -> bool {
        self.0.iter().all(|(k, v)| predicate(k, v))
    }

    /// Removes entries matching the predicate, returning the count removed.
    ///
    /// Parallels `Collection.sweep((v, k) => condition)`.
    pub fn sweep(&mut self, predicate: impl Fn(&K, &V) -> bool) -> usize {
        let before = self.0.len();
        self.0.retain(|k, v| !predicate(k, v));
        before - self.0.len()
    }

    /// Sorts entries by value using the given comparison function.
    /// Returns a Vec of key-value pairs in sorted order.
    ///
    /// Parallels `Collection.sortBy()` or `Collection.sort()`.
    pub fn sort_by(&self, compare: impl Fn(&V, &V) -> Ordering) -> Vec<(K, V)> {
        let mut pairs: Vec<(K, V)> = self.0.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
        pairs.sort_by(|a, b| compare(&a.1, &b.1));
        pairs
    }

    /// Returns the first value in the collection (arbitrary order).
    ///
    /// Parallels `Collection.first()`.
    pub fn first(&self) -> Option<&V> {
        self.0.values().next()
    }

    /// Returns the last value in the collection (arbitrary order).
    ///
    /// Parallels `Collection.last()`.
    pub fn last(&self) -> Option<&V> {
        let mut last = None;
        for v in self.0.values() {
            last = Some(v);
        }
        last
    }

    /// Returns the value at the given index (arbitrary order).
    ///
    /// Parallels `Collection.at(index)`.
    pub fn at(&self, index: usize) -> Option<&V> {
        self.0.values().nth(index)
    }

    /// Returns a random value from the collection.
    ///
    /// Parallels `Collection.random()`.
    /// Uses a simple approach without external rand dependency.
    pub fn random(&self) -> Option<&V> {
        if self.0.is_empty() {
            return None;
        }
        // Use a simple hash-based pseudo-random selection
        let len = self.0.len();
        let index = (len.wrapping_mul(len + 1) / 2) % len;
        self.0.values().nth(index)
    }

    /// Reduces the collection to a single value.
    ///
    /// Parallels `Collection.reduce((acc, v, k) => acc, initial)`.
    pub fn reduce<O>(&self, initial: O, f: impl Fn(O, &K, &V) -> O) -> O {
        self.0.iter().fold(initial, |acc, (k, v)| f(acc, k, v))
    }

    /// Returns a Vec of all values.
    pub fn to_vec(&self) -> Vec<V> {
        self.0.values().cloned().collect()
    }

    /// Returns a Vec of all keys.
    pub fn key_vec(&self) -> Vec<K> {
        self.0.keys().cloned().collect()
    }
}

impl<K, V> Default for Collection<K, V>
where
    K: Eq + Hash + Clone,
    V: Clone,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<K, V> FromIterator<(K, V)> for Collection<K, V>
where
    K: Eq + Hash + Clone,
    V: Clone,
{
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
        Self(HashMap::from_iter(iter))
    }
}

impl<K, V> IntoIterator for Collection<K, V> {
    type Item = (K, V);
    type IntoIter = std::collections::hash_map::IntoIter<K, V>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<K, V> Extend<(K, V)> for Collection<K, V>
where
    K: Eq + Hash,
{
    fn extend<I: IntoIterator<Item = (K, V)>>(&mut self, iter: I) {
        for (k, v) in iter {
            self.0.insert(k, v);
        }
    }
}

impl<K, V> From<HashMap<K, V>> for Collection<K, V> {
    fn from(map: HashMap<K, V>) -> Self {
        Self(map)
    }
}

impl<K, V> From<Collection<K, V>> for HashMap<K, V> {
    fn from(col: Collection<K, V>) -> HashMap<K, V> {
        col.0
    }
}

#[cfg(test)]
mod tests {
    use super::Collection;
    use std::collections::HashMap;

    #[test]
    fn collection_insert_get_remove() {
        let mut col: Collection<String, i32> = Collection::new();
        col.insert("a".to_string(), 1);
        col.insert("b".to_string(), 2);

        assert_eq!(col.get(&"a".to_string()), Some(&1));
        assert_eq!(col.len(), 2);

        col.remove(&"a".to_string());
        assert_eq!(col.len(), 1);
    }

    #[test]
    fn collection_find_and_filter() {
        let mut col: Collection<String, i32> = Collection::new();
        col.insert("a".to_string(), 1);
        col.insert("b".to_string(), 2);
        col.insert("c".to_string(), 3);

        let found = col.find(|_, v| *v > 2);
        assert_eq!(found, Some(&3));

        let filtered = col.filter(|_, v| *v >= 2);
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn collection_partition() {
        let mut col: Collection<String, i32> = Collection::new();
        col.insert("a".to_string(), 1);
        col.insert("b".to_string(), 2);
        col.insert("c".to_string(), 3);

        let (matching, non_matching) = col.partition(|_, v| *v >= 2);
        assert_eq!(matching.len(), 2);
        assert_eq!(non_matching.len(), 1);
    }

    #[test]
    fn collection_sweep_removes_matching() {
        let mut col: Collection<String, i32> = Collection::new();
        col.insert("a".to_string(), 1);
        col.insert("b".to_string(), 2);
        col.insert("c".to_string(), 3);

        let removed = col.sweep(|_, v| *v < 2);
        assert_eq!(removed, 1);
        assert_eq!(col.len(), 2);
    }

    #[test]
    fn collection_map_and_reduce() {
        let mut col: Collection<String, i32> = Collection::new();
        col.insert("a".to_string(), 1);
        col.insert("b".to_string(), 2);

        let doubled: Vec<i32> = col.map(|_, v| *v * 2);
        assert_eq!(doubled.len(), 2);
        assert!(doubled.contains(&2));
        assert!(doubled.contains(&4));

        let sum = col.reduce(0, |acc, _, v| acc + *v);
        assert_eq!(sum, 3);
    }

    #[test]
    fn collection_some_every() {
        let mut col: Collection<String, i32> = Collection::new();
        col.insert("a".to_string(), 1);
        col.insert("b".to_string(), 2);

        assert!(col.some(|_, v| *v > 1));
        assert!(!col.every(|_, v| *v > 1));
    }

    #[test]
    fn collection_sort_by() {
        let mut col: Collection<String, i32> = Collection::new();
        col.insert("a".to_string(), 3);
        col.insert("b".to_string(), 1);
        col.insert("c".to_string(), 2);

        let sorted = col.sort_by(|a, b| a.cmp(b));
        let values: Vec<i32> = sorted.into_iter().map(|(_, v)| v).collect();
        assert_eq!(values, vec![1, 2, 3]);
    }

    #[test]
    fn collection_from_hashmap_and_into() {
        let mut map = HashMap::new();
        map.insert("a".to_string(), 1);
        let col: Collection<String, i32> = Collection::from(map);
        assert_eq!(col.len(), 1);

        let back: HashMap<String, i32> = col.into();
        assert_eq!(back.len(), 1);
    }

    #[test]
    fn collection_from_iterator() {
        let col: Collection<&str, i32> = vec![("a", 1), ("b", 2)].into_iter().collect();
        assert_eq!(col.len(), 2);
    }
}
