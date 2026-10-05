use rpds::RedBlackTreeMapSync;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, ops::Index};

/// Ordered persistent object storage: cloning shares the tree, edits copy only the search path
/// and the edited record. Wire data and schema remain ordinary JSON maps.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
#[schemars(transparent)]
pub struct ObjectMap<K: Ord, V>(#[schemars(with = "BTreeMap<K, V>")] RedBlackTreeMapSync<K, V>);

impl<K: Ord, V> Default for ObjectMap<K, V> {
    fn default() -> Self {
        Self(RedBlackTreeMapSync::new_sync())
    }
}
impl<K: Ord, V> ObjectMap<K, V> {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn len(&self) -> usize {
        self.0.size()
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    pub fn contains_key(&self, key: &K) -> bool {
        self.0.contains_key(key)
    }
    pub fn get(&self, key: &K) -> Option<&V> {
        self.0.get(key)
    }
    pub fn keys(&self) -> impl Iterator<Item = &K> {
        self.0.keys()
    }
    pub fn values(&self) -> impl Iterator<Item = &V> {
        self.0.values()
    }
    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.0.iter()
    }
    pub fn insert(&mut self, key: K, value: V) {
        self.0.insert_mut(key, value);
    }
    pub fn remove(&mut self, key: &K) -> bool {
        self.0.remove_mut(key)
    }
    pub fn clear(&mut self) {
        *self = Self::new();
    }
}
impl<K: Ord + Clone, V: Clone> ObjectMap<K, V> {
    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        self.0.get_mut(key)
    }
}
impl<K: Ord, V> FromIterator<(K, V)> for ObjectMap<K, V> {
    fn from_iter<T: IntoIterator<Item = (K, V)>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}
impl<K: Ord, V, const N: usize> From<[(K, V); N]> for ObjectMap<K, V> {
    fn from(value: [(K, V); N]) -> Self {
        value.into_iter().collect()
    }
}
impl<'a, K: Ord, V> IntoIterator for &'a ObjectMap<K, V> {
    type Item = (&'a K, &'a V);
    type IntoIter = <&'a RedBlackTreeMapSync<K, V> as IntoIterator>::IntoIter;
    fn into_iter(self) -> Self::IntoIter {
        (&self.0).into_iter()
    }
}
impl<K: Ord, V> Index<&K> for ObjectMap<K, V> {
    type Output = V;
    fn index(&self, key: &K) -> &Self::Output {
        self.get(key).expect("missing object identity")
    }
}
