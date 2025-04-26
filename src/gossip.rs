use crate::app::AppState;
use crate::node::{NodeId, NodeState};
use axum::{Json, extract::State};
use rand::SeedableRng;
use rand::prelude::IndexedRandom;
use rand::rngs::SmallRng;
use serde::{Deserialize, Serialize};
use std::{self, collections::HashMap, time::Duration};
use tokio::time::interval;
use tracing::{debug, error, info, instrument, trace};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GossipPayload {
    from: NodeId,
    diffs: HashMap<NodeId, NodeState>,
}

#[instrument]
pub async fn gossip_handler(
    State(app): State<AppState>,
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

#[instrument]
pub async fn start_gossip_loop(app: AppState) -> eyre::Result<()> {
    let client = reqwest::Client::new();
    let mut interval = interval(Duration::from_secs(5));
    let service_info = app.service_info();
    let my_fullname: NodeId = service_info.get_fullname().to_string().into();
    info!("Starting gossip loop...");

    loop {
        interval.tick().await;

        let payload = {
            let nodes = app.nodes();
            let dirty = app.dirty();

            let dirty_ids: Vec<NodeId> = {
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

            let from = app.id().clone();
            GossipPayload { from, diffs }
        };

        if payload.diffs.is_empty() {
            trace!("No diffs to gossip");
            continue;
        }

        let mut rng = SmallRng::from_os_rng();

        let peer_choices: Vec<_> = payload
            .diffs
            .iter()
            .filter(|(fullname, _)| **fullname != my_fullname)
            .collect();

        let sample = peer_choices.choose_multiple(&mut rng, 3);

        for (fullname, node_state) in sample {
            let peer = node_state.address().to_string();
            let peer_url = format!("http://{}/gossip", peer);
            debug!("Gossiping to: {} at {}", fullname, peer);

            let payload_str = serde_json::to_string(&payload).unwrap();
            trace!("Payload: {:?}", payload);

            if let Err(err) = client
                .post(&peer_url)
                .body(payload_str)
                .header("Content-Type", "application/json")
                .header("User-Agent", "Whispers/0.1")
                .timeout(Duration::from_secs(5))
                .send()
                .await
            {
                error!("Failed to send gossip to {}: {:?}", peer, err);
            }
        }
    }
}
