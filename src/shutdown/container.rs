use super::manager::ShutdownManager;
use crate::gossip::GossipState;
use mdns_sd::{ServiceDaemon, ServiceInfo};
use reqwest::Client;
use tokio_util::sync::CancellationToken;

#[derive(Clone)]
pub struct ShutdownContainer {
  pub gossip_state: GossipState,
  pub service_daemon: ServiceDaemon,
  pub domain: String,
  pub service_info: ServiceInfo,
  pub http_client: Client,
}

impl ShutdownContainer {
  pub fn new(
    gossip_state: GossipState,
    service_daemon: ServiceDaemon,
    domain: String,
    service_info: ServiceInfo,
    http_client: Client,
  ) -> Self {
    Self {
      gossip_state,
      service_daemon,
      domain,
      service_info,
      http_client,
    }
  }

  pub async fn spawn<F, Fut>(&self, shutdown: &ShutdownManager, name: &'static str, f: F)
  where
    F: FnOnce(CancellationToken, ShutdownContainer) -> Fut + Send + 'static,
    Fut: Future<Output = eyre::Result<()>> + Send + 'static,
  {
    shutdown.spawn_guarded(name, self, f).await;
  }
}
