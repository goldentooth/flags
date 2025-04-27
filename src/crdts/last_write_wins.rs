use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::hash::Hash;
use std::sync::Arc;

pub trait LastWriteWins {
  fn is_newer_than(&self, other: &Self) -> bool;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LwwMap<K, V>
where
  K: Eq + Hash,
{
  inner: Arc<DashMap<K, V>>,
}

impl<K, V> LwwMap<K, V>
where
  K: Eq + Hash + Clone,
  V: Clone + LastWriteWins,
{
  pub fn new() -> Self {
    Self {
      inner: Arc::new(DashMap::new()),
    }
  }

  pub fn insert(&self, key: K, value: V) {
    match self.inner.get(&key) {
      Some(existing) => {
        if value.is_newer_than(existing.value()) {
          drop(existing);
          self.inner.insert(key, value);
        }
      },
      None => {
        self.inner.insert(key, value);
      },
    }
  }

  pub fn remove(&self, key: &K) {
    self.inner.remove(key);
  }

  pub fn get(&self, key: &K) -> Option<V> {
    self.inner.get(key).map(|v| v.value().clone())
  }

  pub fn iter(&self) -> Vec<(K, V)> {
    self
      .inner
      .iter()
      .map(|entry| (entry.key().clone(), entry.value().clone()))
      .collect()
  }
}
