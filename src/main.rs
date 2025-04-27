use args::parse_args;
use config::build_config;
use gossip::GossipState;
use mdns_sd::ServiceDaemon;
use reqwest::ClientBuilder;
use shutdown::ShutdownManager;
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
  let service_daemon = ServiceDaemon::new()?;
  let gossip_state = GossipState::new(&config.id);

  {
    let gossip_state = gossip_state.clone();
    let service_daemon = service_daemon.clone();
    let domain = config.domain.clone();
    shutdown
      .spawn_guarded("browse_services", move |cancel_token| async move {
        browser::browse_loop(gossip_state, &service_daemon, &domain, cancel_token).await
      })
      .await;
  }

  {
    let service_daemon = service_daemon.clone();
    let service_info = config.service_info()?;
    shutdown
      .spawn_guarded("register_service", move |cancel_token| async move {
        register::register_service(&service_daemon, service_info, cancel_token).await
      })
      .await;
  }

  {
    let gossip_state = gossip_state.clone();
    shutdown
      .spawn_guarded("gossip_listener", move |cancel_token| async move {
        listener::gossip_listen(gossip_state, listener, cancel_token).await
      })
      .await;
  }

  {
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
    shutdown
      .spawn_guarded("gossip_whisper", move |cancel_token| async move {
        whisperer::gossip_whisper(&client, gossip_state, cancel_token).await
      })
      .await;
  }

  {
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
  }

  shutdown.shutdown().await;
  info!("Shutdown complete");
  Ok(())
}
