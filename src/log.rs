use tracing::{Level, subscriber};
use tracing_subscriber::EnvFilter;

pub fn init() -> eyre::Result<()> {
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
  Ok(())
}
