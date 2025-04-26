use mdns_sd::{ServiceDaemon, ServiceInfo};
use tokio_util::sync::CancellationToken;

pub async fn register_service(
  daemon: &ServiceDaemon,
  service_info: ServiceInfo,
  cancel_token: CancellationToken,
) -> eyre::Result<()> {
  daemon.register(service_info)?;

  cancel_token.cancelled().await;

  let shutdown_rx = daemon.shutdown()?;

  if let Some(status) = shutdown_rx.recv_async().await.ok() {
    tracing::info!("mDNS daemon shutdown status: {:?}", status);
  } else {
    tracing::warn!("mDNS daemon shutdown receiver dropped early");
  }

  Ok(())
}
