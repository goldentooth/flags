use crate::crdts::last_write_wins::TrackedLwwMap;
use crate::node::{NodeId, NodeState};
use derivative::Derivative;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GossipPayload {
  pub from: NodeId,
  pub diffs: Vec<(NodeId, NodeState)>,
}

#[derive(Clone, Derivative)]
#[derivative(Debug)]
pub struct GossipState {
  id: NodeId,
  nodes: TrackedLwwMap<NodeId, NodeState>,
}

impl GossipState {
  pub fn new(id: &NodeId) -> Self {
    let id = id.clone();
    let nodes = TrackedLwwMap::new();
    Self { id, nodes }
  }

  pub fn id(&self) -> &NodeId {
    &self.id
  }

  pub fn nodes(&self) -> &TrackedLwwMap<NodeId, NodeState> {
    &self.nodes
  }

  pub async fn add_node(&self, id: &NodeId, node_state: NodeState) {
    self.nodes.insert(id.clone(), node_state).await;
  }

  pub async fn remove_node(&self, id: &NodeId) {
    self.nodes.remove(id).await;
  }
}
