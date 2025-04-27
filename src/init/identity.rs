use super::args::Args;
use super::config::ConfigStage;
use crate::node::NodeId;
use std::net::SocketAddr;
use tokio::net::TcpListener;

pub struct IdentityStage {
  pub args: Args,
  pub listener: TcpListener,
  pub socket_addr: SocketAddr,
}

impl IdentityStage {
  pub fn generate_id(self) -> ConfigStage {
    let id = match &self.args.id {
      Some(id) => NodeId::from(id.clone()),
      None => NodeId::new_random(),
    };

    ConfigStage {
      id,
      domain: self.args.domain.clone(),
      listener: self.listener,
      socket_addr: self.socket_addr,
    }
  }
}
