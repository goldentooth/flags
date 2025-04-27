use super::container::ContainerStage;
use crate::node::NodeId;
use mdns_sd::ServiceInfo;
use std::collections::HashMap;
use std::net::SocketAddr;
use tokio::net::TcpListener;

#[derive(Clone, Debug)]
pub struct Config {
  pub id: NodeId,
  pub domain: String,
  pub socket_addr: SocketAddr,
  pub properties: HashMap<String, String>,
}

impl Config {
  pub fn new(id: &NodeId, domain: &str, socket_addr: SocketAddr) -> Self {
    let id = id.clone();
    let domain = domain.to_string();
    let properties = HashMap::new();

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

pub struct ConfigStage {
  pub id: NodeId,
  pub domain: String,
  pub listener: TcpListener,
  pub socket_addr: SocketAddr,
}

impl ConfigStage {
  pub fn build(self) -> eyre::Result<ContainerStage> {
    let config = Config::new(&self.id, &self.domain, self.socket_addr);
    let service_info = config.service_info()?;
    Ok(ContainerStage {
      config,
      service_info,
      listener: self.listener,
    })
  }
}
