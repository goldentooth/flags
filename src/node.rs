use serde::{Deserialize, Serialize};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use tracing::instrument;

#[derive(PartialEq, Eq, Hash, Debug, Clone, Serialize, Deserialize)]
pub struct NodeId(String);

impl From<String> for NodeId {
    fn from(id: String) -> Self {
        Self(id)
    }
}

impl From<&str> for NodeId {
    fn from(id: &str) -> Self {
        Self(id.to_string())
    }
}

impl std::fmt::Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<NodeId> for String {
    fn from(node_id: NodeId) -> Self {
        node_id.0
    }
}

#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub struct NodeState {
    id: NodeId,
    last_seen: u64,
    address: SocketAddr,
}

impl NodeState {
    pub fn new(id: NodeId, last_seen: u64, address: SocketAddr) -> Self {
        Self {
            id,
            last_seen,
            address,
        }
    }

    pub fn id(&self) -> &NodeId {
        &self.id
    }

    pub fn address(&self) -> &SocketAddr {
        &self.address
    }

    #[instrument]
    pub fn ip(&self) -> eyre::Result<Ipv4Addr> {
        let ip = self.address.ip();
        if let IpAddr::V4(ipv4) = ip {
            return Ok(ipv4);
        }
        eyre::bail!("NodeState::ip() - IP address is not IPv4")
    }

    pub fn port(&self) -> u16 {
        self.address.port()
    }

    pub fn last_seen(&self) -> u64 {
        self.last_seen
    }

    pub fn set_last_seen(&mut self, last_seen: u64) {
        self.last_seen = last_seen;
    }
}
