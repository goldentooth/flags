use crate::gossip::{GossipPayload, GossipState};
use crate::node::NodeId;
use rand::SeedableRng;
use rand::prelude::IndexedRandom;
use rand::rngs::SmallRng;
use reqwest::Client;
use std::{self, collections::HashMap, time::Duration};
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
pub async fn build_gossip_payload(app: &GossipState, full_sync: bool) -> GossipPayload {
  let nodes = app.nodes();
  let diffs = if full_sync {
    nodes
      .iter()
      .map(|entry| (entry.key().clone(), entry.value().clone()))
      .collect()
  } else {
    let dirty_ids: Vec<NodeId> = {
      let dirty = app.dirty();
      let mut dirty = dirty.lock().await;
      let ids = dirty.iter().cloned().collect();
      dirty.clear();
      ids
    };
    let mut diffs = HashMap::new();
    for id in dirty_ids.iter() {
      if let Some(node_state) = nodes.get(id) {
        diffs.insert(id.clone(), node_state.value().clone());
      }
    }
    diffs
  };
  GossipPayload {
    from: app.id().clone(),
    diffs,
  }
}

#[instrument]
pub fn select_gossip_targets(app: &GossipState, count: usize) -> Vec<(NodeId, String)> {
  let nodes = app.nodes();
  let my_id = app.id();
  let targets: Vec<_> = nodes
    .iter()
    .filter(|entry| entry.key() != my_id)
    .map(|entry| (entry.key().clone(), entry.value().address().to_string()))
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
pub async fn gossip_tick(client: &Client, app: &GossipState, full_sync: bool) -> eyre::Result<()> {
  {
    let payload = build_gossip_payload(&app, full_sync).await;
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
  let mut counter: u64 = 0;
  loop {
    let full_sync = counter % 10 == 0;

    tokio::select! {
      biased;
      _ = cancel.cancelled() => {
        debug!("Gossip loop received shutdown");
        break Ok(());
      }
      _ = interval.tick() => {
        trace!("Gossip tick");
        if let Err(error) = gossip_tick(&client, &app, full_sync).await {
          trace!("Error in gossip tick: {}", error);
          continue;
        }
      }
    }

    counter += 1;
  }
}
