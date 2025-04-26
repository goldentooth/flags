use crate::gossip::{GossipPayload, GossipState};
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

use tracing::{debug, instrument, trace};

pub async fn gossip_listen(
  gossip_state: GossipState,
  listener: TcpListener,
  cancel_token: CancellationToken,
) -> eyre::Result<()> {
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
  println!("Gossip listener shutdown");
  Ok(())
}

async fn shutdown_signal(cancel_token: CancellationToken) {
  println!("Waiting for shutdown signal...");
  cancel_token.cancelled().await;
  println!("Shutdown signal received!");
}

#[instrument]
pub async fn gossip_handler(
  State(app): State<GossipState>,
  Json(payload): Json<GossipPayload>,
) -> &'static str {
  let nodes = app.nodes();
  debug!("Received gossip from: {}", payload.from);

  for (key, incoming) in payload.diffs {
    trace!("Processing state for: {}", key);

    if let Some(existing) = nodes.get(&key) {
      if incoming.last_seen() > existing.last_seen() {
        trace!("Updating state for: {}", key);
        nodes.insert(key, incoming);
      }
    } else {
      trace!("Adding new state for: {}", key);
      nodes.insert(key, incoming);
    }
  }

  "ok"
}
