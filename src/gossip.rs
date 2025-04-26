use crate::node::{NodeId, NodeState};
use dashmap::DashMap;
use derivative::Derivative;
use serde::{Deserialize, Serialize};
use std::{
  self,
  collections::{HashMap, HashSet},
  sync::Arc,
};
use tokio::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GossipPayload {
  pub from: NodeId,
  pub diffs: HashMap<NodeId, NodeState>,
}

#[derive(Clone, Derivative)]
#[derivative(Debug)]
pub struct GossipState {
  id: NodeId,
  nodes: Arc<DashMap<NodeId, NodeState>>,
  dirty: Arc<Mutex<HashSet<NodeId>>>,
}

impl GossipState {
  pub fn new(id: &NodeId) -> Self {
    let id = id.clone();
    let nodes = Arc::new(DashMap::new());
    let dirty = Arc::new(Mutex::new(HashSet::new()));
    Self { id, nodes, dirty }
  }

  pub fn id(&self) -> &NodeId {
    &self.id
  }

  pub fn nodes(&self) -> Arc<DashMap<NodeId, NodeState>> {
    Arc::clone(&self.nodes)
  }

  pub fn dirty(&self) -> Arc<Mutex<HashSet<NodeId>>> {
    Arc::clone(&self.dirty)
  }

  pub async fn add_node(&self, id: &NodeId, node_state: NodeState) {
    let mut is_dirty = true;
    if let Some(existing) = self.nodes.get(id) {
      is_dirty = existing.value() != &node_state;
    }
    self.nodes.insert(id.clone(), node_state);
    if is_dirty {
      let mut dirty = self.dirty.lock().await;
      dirty.insert(id.clone());
    }
  }

  pub async fn remove_node(&self, id: &NodeId) {
    self.nodes.remove(id);
    let mut dirty = self.dirty.lock().await;
    dirty.insert(id.clone());
  }
}
