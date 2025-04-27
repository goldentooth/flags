use crate::gossip::GossipState;
use mdns_sd::{ServiceDaemon, ServiceInfo};
use reqwest::Client;

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
}
