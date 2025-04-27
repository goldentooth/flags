use args::parse_args;
use config::build_config;
use gossip::GossipState;
use mdns_sd::ServiceDaemon;
use reqwest::ClientBuilder;
use shutdown::{container::ShutdownContainer, manager::ShutdownManager};
use std::time::Duration;
use tokio::signal;
use tracing::{info, instrument};

mod args;
mod browser;
mod config;
mod crdts;
mod gossip;
mod listener;
mod log;
mod node;
mod register;
mod shutdown;
mod whisperer;

#[tokio::main]
#[instrument]
async fn main() -> eyre::Result<()> {
  if false {
    log::init()?;
  } else {
    console_subscriber::init();
  }

  let args = parse_args()?;
  let (config, listener) = build_config(args).await?;
  let shutdown = ShutdownManager::new();
  let container = {
    let gossip_state = GossipState::new(&config.id);
    let service_daemon = ServiceDaemon::new()?;
    let domain = config.domain.clone();
    let service_info = config.service_info()?;
    let client = ClientBuilder::new()
      .timeout(Duration::from_secs(5))
      .connect_timeout(Duration::from_secs(1))
      .pool_idle_timeout(Duration::from_secs(1))
      .pool_max_idle_per_host(0)
      .http2_keep_alive_interval(None)
      .tcp_keepalive(None)
      .build()
      .expect("Failed to create HTTP client");
    ShutdownContainer::new(gossip_state, service_daemon, domain, service_info, client)
  };

  container.register_tasks(&shutdown, listener).await;

  shutdown
    .spawn("ctrl_c", {
      let shutdown = shutdown.clone();
      async move {
        signal::ctrl_c().await.expect("failed to listen for event");
        info!("Ctrl-C pressed, shutting down...");
        shutdown.cancel();
      }
    })
    .await;

  shutdown.shutdown().await;
  info!("Shutdown complete");
  Ok(())
}
