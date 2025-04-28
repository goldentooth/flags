use super::state::{GossipPayload, GossipState};
use crate::shutdown::container::ShutdownContainer;
use axum::{Json, extract::State};
use axum::{
  Router,
  routing::{get, post},
};
use serde_json::json;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;
use tower::ServiceBuilder;
use tower_http::{timeout::TimeoutLayer, trace::TraceLayer};

use tracing::{debug, instrument};

pub async fn gossip_listen(
  container: &ShutdownContainer,
  listener: TcpListener,
  cancel_token: CancellationToken,
) -> eyre::Result<()> {
  let gossip_state = container.gossip_state.clone();
  let layer = ServiceBuilder::new()
    .layer(TraceLayer::new_for_http())
    .layer(TimeoutLayer::new(Duration::from_secs(5)));
  let app = Router::new()
    .route("/gossip", post(gossip_handler))
    .route("/health", get(|| async { Json(json!({"status": "ok"})) }))
    .layer(layer)
    .with_state(gossip_state);
  axum::serve(listener, app)
    .with_graceful_shutdown(shutdown_signal(cancel_token))
    .await
    .unwrap();
  Ok(())
}

async fn shutdown_signal(cancel_token: CancellationToken) {
  cancel_token.cancelled().await;
}

#[instrument]
pub async fn gossip_handler(
  State(app): State<GossipState>,
  Json(payload): Json<GossipPayload>,
) -> &'static str {
  let nodes = app.nodes();
  debug!("Received gossip from: {}", payload.from);
  for (key, incoming) in payload.diffs {
    nodes.insert(key, incoming).await;
  }
  "ok"
}
