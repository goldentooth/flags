use crate::gossip::GossipState;
use crate::node::{NodeId, NodeState};
use mdns_sd::{IfKind, ServiceDaemon, ServiceEvent, ServiceInfo};
use std::net::SocketAddr;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, instrument, trace};

#[derive(Debug, Clone)]
pub struct BrowserDelegate {
  pub gossip_state: GossipState,
}

impl BrowserDelegate {
  pub fn new(gossip_state: GossipState) -> Self {
    let gossip_state = gossip_state.clone();
    Self { gossip_state }
  }

  pub async fn handle_event(&mut self, event: ServiceEvent) -> eyre::Result<()> {
    match event {
      ServiceEvent::SearchStarted(service_type) => {
        self.did_start_search(service_type);
      },
      ServiceEvent::ServiceFound(service_type, fullname) => {
        self.did_find_service(service_type, fullname)?;
      },
      ServiceEvent::ServiceResolved(service_info) => {
        self.did_resolve_service(service_info).await?;
      },
      ServiceEvent::ServiceRemoved(service_type, fullname) => {
        self.did_remove_service(service_type, fullname).await?;
      },
      ServiceEvent::SearchStopped(service_type) => {
        self.did_stop_search(service_type);
      },
    }
    Ok(())
  }

  pub fn did_start_search(&self, service_type: String) {
    trace!("Started browsing for services of type: {}", service_type);
  }

  pub fn did_find_service(&self, service_type: String, fullname: String) -> eyre::Result<()> {
    debug!("Found service of type {}: {}", service_type, fullname);
    Ok(())
  }

  #[instrument]
  pub async fn did_resolve_service(&mut self, service_info: ServiceInfo) -> eyre::Result<()> {
    debug!("Resolved service: {:?}", service_info);
    let id = service_info.get_node_id()?;
    let socket_addr = service_info.get_socket_addr()?;
    let last_seen = SystemTime::now()
      .duration_since(UNIX_EPOCH)
      .expect("Time went backwards")
      .as_secs();
    let node_state = NodeState::new(&id, last_seen, socket_addr);
    self.gossip_state.add_node(&id, node_state).await;
    Ok(())
  }

  pub async fn did_remove_service(&self, service_type: String, id: String) -> eyre::Result<()> {
    debug!("Removed service of type {}: {}", service_type, id);
    let id = NodeId::from(id);
    self.gossip_state.remove_node(&id).await;
    Ok(())
  }

  pub fn did_stop_search(&self, service_type: String) {
    debug!("Stopped browsing for services of type: {}", service_type);
  }

  pub fn handle_error(&self, error: eyre::Error) -> eyre::Result<()> {
    error!("Error in browse loop: {}", error);
    Ok(())
  }
}

pub trait ServiceInfoExt {
  fn get_node_id(&self) -> eyre::Result<NodeId>;
  fn get_socket_addr(&self) -> eyre::Result<SocketAddr>;
}

impl ServiceInfoExt for ServiceInfo {
  fn get_node_id(&self) -> eyre::Result<NodeId> {
    self
      .get_properties()
      .get("node.id")
      .map(|v| v.val_str())
      .map(NodeId::from)
      .ok_or_else(|| eyre::eyre!("Node ID not found in service properties"))
  }

  fn get_socket_addr(&self) -> eyre::Result<SocketAddr> {
    let ip = self
      .get_addresses_v4()
      .iter()
      .cloned()
      .next()
      .ok_or_else(|| eyre::eyre!("IP address not found in service info"))?;
    let port = self.get_port();
    Ok(SocketAddr::new((*ip).into(), port))
  }
}

pub async fn browse_loop(
  gossip_state: GossipState,
  service_daemon: &ServiceDaemon,
  service_type: &str,
  cancel_token: CancellationToken,
) -> eyre::Result<()> {
  let mut delegate = BrowserDelegate::new(gossip_state);
  service_daemon.disable_interface(IfKind::IPv6)?;
  let receiver = service_daemon.browse(&service_type)?;
  loop {
    tokio::select! {
        biased;
        _ = cancel_token.cancelled() => {
            debug!("Browse task received shutdown signal");
            service_daemon.stop_browse(service_type)?;
            debug!("Stopped browsing for service type: {}", service_type);
            break Ok(());
        }
        recv = receiver.recv_async() => {
            match recv {
                Ok(event) => delegate.handle_event(event).await?,
                Err(error) => delegate.handle_error(error.into())?,
            }
        }
    }
  }
}
