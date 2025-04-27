use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, hash::Hash, mem, sync::Arc};
use tokio::sync::Mutex;

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

#[derive(Debug, Clone)]
pub struct TrackedLwwMap<K, V>
where
  K: Eq + Hash,
{
  map: LwwMap<K, V>,
  dirty: Arc<Mutex<HashSet<K>>>,
}

impl<K, V> TrackedLwwMap<K, V>
where
  K: Eq + Hash + Clone,
  V: Clone + LastWriteWins,
{
  pub fn new() -> Self {
    Self {
      map: LwwMap::new(),
      dirty: Arc::new(Mutex::new(HashSet::new())),
    }
  }

  pub async fn insert(&self, key: K, value: V) {
    let changed = {
      let current = self.map.get(&key);
      match current {
        Some(existing) => value.is_newer_than(&existing),
        None => true,
      }
    };

    if changed {
      self.map.insert(key.clone(), value);
      let mut dirty = self.dirty.lock().await;
      dirty.insert(key);
    }
  }

  pub async fn remove(&self, key: &K) {
    self.map.remove(key);
    let mut dirty = self.dirty.lock().await;
    dirty.insert(key.clone());
  }

  pub async fn take_dirty(&self) -> HashSet<K> {
    let mut dirty = self.dirty.lock().await;
    mem::take(&mut *dirty)
  }

  #[allow(dead_code)]
  pub async fn take_dirty_batch(&self, count: usize) -> HashSet<K> {
    let mut dirty = self.dirty.lock().await;
    let mut taken = HashSet::with_capacity(count.min(dirty.len()));

    for key in dirty.iter().take(count).cloned().collect::<Vec<_>>() {
      dirty.remove(&key);
      taken.insert(key);
    }

    taken
  }

  pub fn get(&self, key: &K) -> Option<V> {
    self.map.get(key)
  }

  pub fn iter(&self) -> Vec<(K, V)> {
    self.map.iter()
  }
}
