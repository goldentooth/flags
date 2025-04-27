use crate::shutdown::container::ShutdownContainer;
use tokio_util::sync::CancellationToken;

pub async fn register_service(
  container: &ShutdownContainer,
  cancel_token: CancellationToken,
) -> eyre::Result<()> {
  let daemon = container.service_daemon.clone();
  let service_info = container.service_info.clone();
  daemon.register(service_info)?;
  cancel_token.cancelled().await;
  let shutdown_rx = daemon.shutdown()?;
  if let Ok(status) = shutdown_rx.recv_async().await {
    tracing::info!("mDNS daemon shutdown status: {:?}", status);
  } else {
    tracing::warn!("mDNS daemon shutdown receiver dropped early");
  }

  Ok(())
}
