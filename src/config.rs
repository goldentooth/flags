use crate::args::Args;
use crate::node::NodeId;
use local_ip_address::local_ip;
use mdns_sd::ServiceInfo;
use std::{
  collections::HashMap,
  net::{IpAddr, Ipv4Addr, SocketAddr},
};
use tokio::net::TcpListener;
use tracing::{info, instrument, trace};

const SERVICE_TYPE: &str = "_whispers._tcp.local.";

#[derive(Clone, Debug)]
pub struct Config {
  pub id: NodeId,
  pub domain: String,
  pub socket_addr: SocketAddr,
  pub properties: HashMap<String, String>,
}

impl Config {
  pub fn new(id: &NodeId, socket_addr: SocketAddr) -> Self {
    let id = id.clone();
    let properties = HashMap::new();
    let domain = SERVICE_TYPE.to_string();

    Self {
      id,
      domain,
      socket_addr,
      properties,
    }
  }

  pub fn service_info(&self) -> eyre::Result<ServiceInfo> {
    let socket_addr = self.socket_addr;

    let mut properties = self.properties.clone();
    properties.insert("node.id".to_string(), self.id.clone().into());
    properties.insert("node.ip".to_string(), socket_addr.ip().to_string());
    properties.insert("node.port".to_string(), socket_addr.port().to_string());
    properties.insert("node.address".to_string(), socket_addr.to_string());

    let service_info = ServiceInfo::new(
      &self.domain,
      &String::from(self.id.clone()),
      &format!("{}.local.", self.id),
      socket_addr.ip().to_string(),
      socket_addr.port(),
      properties.clone(),
    )
    .map_err(|e| eyre::eyre!(e))?;
    Ok(service_info)
  }
}

pub async fn bind_socket(ip: Ipv4Addr, port: u16) -> eyre::Result<(TcpListener, SocketAddr)> {
  let addr = SocketAddr::new(ip.into(), port);
  let listener = TcpListener::bind(addr).await?;
  let bound_addr = listener.local_addr()?;
  Ok((listener, bound_addr))
}

#[instrument]
pub async fn build_config(args: Args) -> eyre::Result<(Config, TcpListener)> {
  let id: NodeId = args
    .id
    .clone()
    .unwrap_or_else(|| {
      let id = uuid::Uuid::new_v4().to_string();
      trace!("Generated new node ID: {}", id);
      id
    })
    .into();

  let ip = {
    if let Some(ip) = args.ip.clone() {
      ip.parse::<Ipv4Addr>()?
    } else if let Ok(IpAddr::V4(ip)) = local_ip() {
      ip
    } else {
      eyre::bail!("Could not get local IP address")
    }
  };

  let port = args.port;
  let (listener, socket_addr) = bind_socket(ip, port).await?;
  info!("Listening on: {}", socket_addr);

  let config = Config::new(&id, socket_addr);

  Ok((config, listener))
}
