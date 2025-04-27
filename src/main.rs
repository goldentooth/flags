use args::parse_args;
use config::build_config;
use mdns_sd::ServiceDaemon;
use reqwest::ClientBuilder;
use shutdown::ShutdownManager;
use std::time::Duration;
use tokio::signal;
use tracing::{info, instrument};

mod args;
mod browser;
mod config;
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
  println!("Is shutdown: {}", shutdown.is_shutdown());
  let service_daemon = ServiceDaemon::new()?;
  let gossip_state = gossip::GossipState::new(&config.id);
  let cancel_token = shutdown.cancel_token();

  shutdown
    .spawn("browse", {
      let gossip_state = gossip_state.clone();
      let service_daemon = service_daemon.clone();
      let domain = config.domain.clone();
      async move {
        browser::browse_loop(gossip_state, &service_daemon, &domain, cancel_token)
          .await
          .unwrap();
      }
    })
    .await;

  let cancel_token = shutdown.cancel_token();
  shutdown
    .spawn("register_service", {
      let service_daemon = service_daemon.clone();
      let service_info = config.service_info()?;
      async move {
        register::register_service(&service_daemon, service_info, cancel_token)
          .await
          .expect("Failed to register service");
      }
    })
    .await;

  let cancel_token = shutdown.cancel_token();
  shutdown
    .spawn("gossip_listener", {
      let gossip_state = gossip_state.clone();
      async move {
        listener::gossip_listen(gossip_state, listener, cancel_token)
          .await
          .expect("Failed to start gossip listener");
      }
    })
    .await;

  let cancel_token = shutdown.cancel_token();
  shutdown
    .spawn("gossip_whisper", {
      let gossip_state = gossip_state.clone();
      let client = ClientBuilder::new()
        .timeout(Duration::from_secs(5))
        .connect_timeout(Duration::from_secs(1))
        .pool_idle_timeout(Duration::from_secs(1))
        .pool_max_idle_per_host(0)
        .http2_keep_alive_interval(None)
        .tcp_keepalive(None)
        .build()
        .expect("Failed to create HTTP client");
      async move {
        whisperer::gossip_whisper(&client, gossip_state, cancel_token)
          .await
          .expect("Failed to start gossip whisperer");
      }
    })
    .await;

  let shutdown_clone = shutdown.clone();
  shutdown
    .spawn("ctrl_c", {
      async move {
        signal::ctrl_c().await.expect("failed to listen for event");
        info!("Ctrl-C pressed, shutting down...");
        shutdown_clone.cancel();
      }
    })
    .await;

  shutdown.shutdown().await;
  info!("Shutdown complete");
  Ok(())
}
