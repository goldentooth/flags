use clap::Parser;
use tracing::{instrument, trace};

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
}

#[instrument]
pub fn parse_args() -> eyre::Result<Args> {
  let args = Args::parse();
  trace!("Running with arguments: {:?}", args);

  if args.port == 0 {
    trace!("Port is 0; binding will be decided by the OS");
  }

  Ok(args)
}
