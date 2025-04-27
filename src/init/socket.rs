use super::args::Args;
use super::identity::IdentityStage;
use std::net::{Ipv4Addr, SocketAddr};
use tokio::net::TcpListener;

pub struct SocketStage {
  pub args: Args,
  pub ip: Ipv4Addr,
}

impl SocketStage {
  pub async fn bind(self) -> eyre::Result<IdentityStage> {
    let addr = SocketAddr::new(self.ip.into(), self.args.port);
    let listener = TcpListener::bind(addr).await?;
    let socket_addr = listener.local_addr()?;
    Ok(IdentityStage {
      args: self.args,
      listener,
      socket_addr,
    })
  }
}
