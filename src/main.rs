use app::{AppConfig, AppState};
use clap::Parser;
use local_ip_address::local_ip;
use mdns_sd::{ServiceDaemon, ServiceInfo};
use service::Service;
use std::{
    self,
    collections::HashMap,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::Arc,
};
use tokio::{net::TcpListener, sync::Notify};
use tracing::{Level, debug, info, instrument, subscriber};
use tracing_subscriber::EnvFilter;

use uuid::Uuid;

mod app;
mod gossip;
mod node;
mod service;

const SERVICE_TYPE: &str = "_whispers._tcp.local.";

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// IP address of the node
    #[arg(short, long)]
    ip: Option<String>,
    /// Port of the node
    #[arg(short, long, default_value_t = 0)]
    port: u16,
}

#[tokio::main]
#[instrument]
async fn main() -> eyre::Result<()> {
    // Configure a custom event formatter
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_env_filter(EnvFilter::from_default_env())
        .with_level(true)
        .with_target(false)
        .with_line_number(true)
        .with_file(true)
        .with_ansi(true)
        .with_thread_ids(false)
        .with_thread_names(false)
        .without_time()
        .compact()
        .finish();
    subscriber::set_global_default(subscriber)?;

    let args = Args::parse();
    info!("Running with arguments: {:?}", args);
    let id = Uuid::new_v4().to_string();
    info!("Node ID: {}", id);
    let ip = {
        if let Some(ip) = args.ip {
            ip.parse::<Ipv4Addr>()?
        } else if let Ok(IpAddr::V4(ip)) = local_ip() {
            ip
        } else {
            eyre::bail!("Could not get local IP address")
        }
    };
    info!("Node IP: {}", ip);
    let port = args.port;
    info!("Node Port: {}", port);
    let (listener, socket_addr) = bind_socket(ip, port).await?;
    info!("Listening on: {}", socket_addr);

    let mut properties = HashMap::new();
    properties.insert("node.id".to_string(), id.clone());
    properties.insert("node.ip".to_string(), ip.to_string());
    properties.insert("node.port".to_string(), port.to_string());
    properties.insert("node.type".to_string(), SERVICE_TYPE.to_string());

    info!("Node Properties: {:?}", properties);

    let config = AppConfig {
        id: id.into(),
        domain: SERVICE_TYPE.to_string(),
        socket_addr,
        properties,
    };

    info!("Node Config: {:?}", config);

    let service_info = create_service_info(&config)?;
    let service_daemon = ServiceDaemon::new()?;
    let state = AppState::new(config.id.clone(), service_info, service_daemon);

    debug!("Starting node service...");
    let mut service = Service::new(state);
    debug!("Starting listening...");
    let _listener = service.listen(listener).await?;
    debug!("Registering service...");
    let _registrar = service.register().await?;
    debug!("Starting whispering...");
    let _whisperer = service.whisper().await?;
    debug!("Starting browsing...");
    let _browser = service.browse(SERVICE_TYPE).await?;
    debug!("Startup complete.");

    let notify = Arc::new(Notify::new());
    let notify_clone = Arc::clone(&notify);

    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.unwrap();
        info!("Received shutdown signal");
        notify_clone.notify_waiters(); // Wake up anyone waiting
    });

    notify.notified().await;
    debug!("Shutting down...");
    debug!("Stopping registrar...");
    _registrar.abort();
    debug!("Stopping whisperer...");
    _whisperer.abort();
    debug!("Stopping listener...");
    _listener.abort();
    debug!("Stopping browser...");
    _browser.abort();
    info!("Goodbye!");
    Ok(())
}

pub async fn bind_socket(ip: Ipv4Addr, port: u16) -> eyre::Result<(TcpListener, SocketAddr)> {
    let addr = SocketAddr::new(ip.into(), port);
    let listener = TcpListener::bind(addr).await?;
    let bound_addr = listener.local_addr()?;
    Ok((listener, bound_addr))
}

fn create_service_info(config: &AppConfig) -> eyre::Result<ServiceInfo> {
    ServiceInfo::new(
        &config.domain,
        &config.id.to_string(),
        &format!("{}.local.", config.id),
        config.socket_addr.ip().to_string(),
        config.socket_addr.port(),
        config.properties.clone(),
    )
    .map_err(|e| eyre::eyre!(e))
}
