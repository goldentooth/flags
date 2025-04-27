use crate::gossip::{GossipPayload, GossipState};
use crate::node::NodeId;
use rand::SeedableRng;
use rand::prelude::IndexedRandom;
use rand::rngs::SmallRng;
use reqwest::Client;
use std::{self, time::Duration};
use tokio::time::interval;
use tokio_util::sync::CancellationToken;
use tracing::{debug, info, instrument, trace};

#[instrument]
pub async fn is_node_healthy(client: &Client, target_addr: &str) -> bool {
  let url = format!("http://{}/health", target_addr);
  match client
    .get(&url)
    .timeout(Duration::from_secs(1))
    .send()
    .await
  {
    Ok(resp) if resp.status().is_success() => true,
    _ => false,
  }
}

#[instrument]
pub async fn build_gossip_payload(state: &GossipState) -> GossipPayload {
  let dirty_keys = state.nodes().take_dirty().await;
  let diffs: Vec<_> = dirty_keys
    .into_iter()
    .filter_map(|id| state.nodes().get(&id).map(|v| (id, v)))
    .collect();

  GossipPayload {
    from: state.id().clone(),
    diffs,
  }
}

#[instrument]
pub fn select_gossip_targets(app: &GossipState, count: usize) -> Vec<(NodeId, String)> {
  let nodes = app.nodes();
  let my_id = app.id();
  let targets: Vec<_> = nodes
    .iter()
    .into_iter()
    .filter(|entry| entry.0 != *my_id)
    .map(|entry| (entry.0, entry.1.address().to_string()))
    .collect();
  let mut rng = SmallRng::from_os_rng();
  targets.choose_multiple(&mut rng, count).cloned().collect()
}

#[instrument]
async fn send_gossip(
  client: &Client,
  target_addr: &str,
  payload: &GossipPayload,
) -> eyre::Result<()> {
  let url = format!("http://{}/gossip", target_addr);
  let payload_str = serde_json::to_string(payload)?;
  client
    .post(&url)
    .body(payload_str)
    .header("Content-Type", "application/json")
    .send()
    .await?
    .error_for_status()?;
  Ok(())
}

#[instrument]
pub async fn gossip_tick(client: &Client, app: &GossipState) -> eyre::Result<()> {
  {
    let payload = build_gossip_payload(&app).await;
    if payload.diffs.is_empty() {
      eyre::bail!("No gossip to send");
    }

    let targets = select_gossip_targets(&app, 3);
    if targets.is_empty() {
      eyre::bail!("No gossip targets found");
    }

    for (id, address) in targets {
      if !is_node_healthy(client, &address).await {
        eyre::bail!("Node {} is not healthy", id);
      }

      send_gossip(&client, &address, &payload)
        .await
        .map_err(|error| {
          debug!("Failed to send gossip to {}: {} ({:?})", id, error, error);
          eyre::eyre!("Failed to send gossip to {}: {}", id, error)
        })?;
    }
    Ok(())
  }
}

#[instrument]
pub async fn gossip_whisper(
  client: &Client,
  app: GossipState,
  cancel: CancellationToken,
) -> eyre::Result<()> {
  let mut interval = interval(Duration::from_secs(5));
  info!("Starting gossip loop...");
  loop {
    tokio::select! {
      biased;
      _ = cancel.cancelled() => {
        debug!("Gossip loop received shutdown");
        break Ok(());
      }
      _ = interval.tick() => {
        trace!("Gossip tick");
        if let Err(error) = gossip_tick(&client, &app).await {
          trace!("Error in gossip tick: {}", error);
          continue;
        }
      }
    }
  }
}
