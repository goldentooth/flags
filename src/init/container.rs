use super::config::Config;
use crate::gossip::GossipState;
use crate::shutdown::container::ShutdownContainer;
use mdns_sd::{ServiceDaemon, ServiceInfo};
use reqwest::ClientBuilder;
use std::time::Duration;
use tokio::net::TcpListener;

pub struct ContainerStage {
  pub config: Config,
  pub service_info: ServiceInfo,
  pub listener: TcpListener,
}

impl ContainerStage {
  pub fn finalize(self) -> (ShutdownContainer, TcpListener) {
    let gossip_state = GossipState::new(&self.config.id);
    let service_daemon = ServiceDaemon::new().expect("Failed to create service daemon");
    let domain = self.config.domain.clone();
    let client = ClientBuilder::new()
      .timeout(Duration::from_secs(5))
      .connect_timeout(Duration::from_secs(1))
      .pool_idle_timeout(Duration::from_secs(1))
      .pool_max_idle_per_host(0)
      .http2_keep_alive_interval(None)
      .tcp_keepalive(None)
      .build()
      .expect("Failed to build HTTP client");

    let container = ShutdownContainer::new(
      gossip_state,
      service_daemon,
      domain,
      self.service_info,
      client,
    );

    (container, self.listener)
  }
}
