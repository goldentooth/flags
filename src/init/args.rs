use super::socket::SocketStage;
use clap::Parser;
use std::net::IpAddr;
use tracing::{instrument, trace};

const SERVICE_TYPE: &str = "_whispers._tcp.local.";

#[derive(Parser, Clone, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
  /// IP address of the node; normally auto-detected
  #[arg(short, long)]
  pub ip: Option<String>,
  /// UUID of the node; normally auto-generated
  #[arg(long)]
  pub id: Option<String>,
  /// Port of the node; normally picked by the OS
  #[arg(short, long, default_value_t = 0)]
  pub port: u16,
  /// The service type (domain, like "_whispers._tcp.local.") to advertise
  #[arg(short, long, default_value_t = String::from(SERVICE_TYPE))]
  pub domain: String,
}

#[derive(Debug)]
pub struct ArgsStage {
  pub args: Args,
}

impl ArgsStage {
  #[instrument]
  pub fn parse() -> eyre::Result<Self> {
    let args = Args::parse();
    trace!("Running with arguments: {:?}", args);

    if args.port == 0 {
      trace!("Port is 0; binding will be decided by the OS");
    }

    Ok(Self { args })
  }

  #[instrument]
  pub fn bind_socket(self) -> eyre::Result<SocketStage> {
    let ip = match &self.args.ip {
      Some(ip) => ip.parse()?,
      None => {
        if let Ok(IpAddr::V4(ip)) = local_ip_address::local_ip() {
          ip
        } else {
          return Err(eyre::eyre!("Could not get local IP address"));
        }
      },
    };

    Ok(SocketStage {
      args: self.args,
      ip,
    })
  }
}
