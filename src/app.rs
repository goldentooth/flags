use crate::node::{NodeId, NodeState};
use dashmap::DashMap;
use derivative::Derivative;
use mdns_sd::{ServiceDaemon, ServiceInfo};
use std::{
    self,
    collections::{HashMap, HashSet},
    net::{Ipv4Addr, SocketAddr},
    sync::Arc,
};
use tokio::sync::Mutex;
use tracing::instrument;

#[derive(Clone, Derivative)]
#[derivative(Debug)]
pub struct AppState {
    id: NodeId,
    #[derivative(Debug = "ignore")]
    service_daemon: ServiceDaemon,
    service_info: ServiceInfo,
    nodes: Arc<DashMap<NodeId, NodeState>>,
    dirty: Arc<Mutex<HashSet<NodeId>>>,
}

impl AppState {
    pub fn new(id: NodeId, service_info: ServiceInfo, service_daemon: ServiceDaemon) -> Self {
        let nodes = Arc::new(DashMap::new());
        let dirty = Arc::new(Mutex::new(HashSet::new()));
        let service_info = service_info.clone();
        Self {
            id,
            service_daemon,
            service_info,
            nodes,
            dirty,
        }
    }

    pub fn id(&self) -> &NodeId {
        &self.id
    }

    pub fn service_info(&self) -> &ServiceInfo {
        &self.service_info
    }

    pub fn service_daemon(&self) -> &ServiceDaemon {
        &self.service_daemon
    }

    pub fn nodes(&self) -> Arc<DashMap<NodeId, NodeState>> {
        Arc::clone(&self.nodes)
    }

    pub fn dirty(&self) -> Arc<Mutex<HashSet<NodeId>>> {
        Arc::clone(&self.dirty)
    }

    pub fn port(&self) -> u16 {
        self.service_info.get_port()
    }

    #[instrument]
    pub fn address(&self) -> eyre::Result<SocketAddr> {
        let ip = self.ip()?;
        let port = self.port();
        let result = SocketAddr::new(ip.into(), port);
        Ok(result)
    }

    #[instrument]
    pub fn ip(&self) -> eyre::Result<Ipv4Addr> {
        self.service_info()
            .get_addresses_v4()
            .iter()
            .cloned()
            .next()
            .cloned()
            .ok_or_else(|| eyre::eyre!("No IPv4 address found"))
    }

    pub async fn add_node(&self, id: NodeId, node_state: NodeState) {
        self.nodes.insert(id.clone(), node_state);
        let mut dirty = self.dirty.lock().await;
        dirty.insert(id);
    }

    pub async fn remove_node(&self, id: &NodeId) {
        self.nodes.remove(id);
        let mut dirty = self.dirty.lock().await;
        dirty.insert(id.clone());
    }
}

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub id: NodeId,
    pub domain: String,
    pub socket_addr: SocketAddr,
    pub properties: HashMap<String, String>,
}
