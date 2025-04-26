use crate::app::AppState;
use crate::gossip;
use crate::node::{NodeId, NodeState};
use axum::{Router, routing::post};
use mdns_sd::ServiceEvent;
use std::{self, net::SocketAddr};
use tokio::{net::TcpListener, task::JoinHandle};
use tracing::{debug, error, info, instrument};

#[derive(Debug, Clone)]
pub struct Service {
    state: AppState,
}

impl Service {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }

    pub fn state(&self) -> &AppState {
        &self.state
    }

    #[instrument]
    pub async fn register(&mut self) -> eyre::Result<JoinHandle<()>> {
        let state = self.state.clone();
        let handle = tokio::spawn(async move {
            let service_info = state.service_info();
            let fullname = service_info.get_fullname().to_string();
            let service_daemon = state.service_daemon();
            if let Err(error) = service_daemon.register(service_info.clone()) {
                error!("Failed to register service: {}", error);
            } else {
                debug!("Service registered: {}", fullname);
            }
        });
        Ok(handle)
    }

    #[instrument]
    pub async fn whisper(&mut self) -> eyre::Result<JoinHandle<()>> {
        let state = self.state.clone();
        let handle = tokio::spawn(async move {
            if let Err(error) = gossip::start_gossip_loop(state).await {
                error!("Failed to start gossip loop: {}", error);
            }
        });
        Ok(handle)
    }

    #[instrument]
    pub async fn listen(&mut self, listener: TcpListener) -> eyre::Result<JoinHandle<()>> {
        let state = self.state.clone();
        let handle = tokio::spawn(async move {
            let app = Router::new()
                .route("/gossip", post(gossip::gossip_handler))
                .with_state(state);
            if let Err(error) = axum::serve(listener, app)
                .await
                .map_err(|e| eyre::eyre!("Failed to start server: {}", e))
            {
                error!("Failed to start server: {}", error);
            }
        });
        Ok(handle)
    }

    #[instrument]
    pub async fn browse(&mut self, service_type: &str) -> eyre::Result<JoinHandle<()>> {
        let service_type = service_type.to_string();
        debug!("Browsing for services of type: {}", service_type);
        let receiver = self.state().service_daemon().browse(&service_type)?;
        let state = self.state.clone();
        let handle = tokio::spawn(async move {
            while let Ok(event) = receiver.recv_async().await {
                match event {
                    ServiceEvent::ServiceResolved(service_info) => {
                        if let Some(id) = service_info
                            .get_properties()
                            .get("node.id")
                            .map(|v| v.to_string())
                        {
                            debug!("Resolved service: {}", id);
                            if let Some(ip) = service_info.get_addresses_v4().iter().cloned().next()
                            {
                                let port = service_info.get_port();
                                let addr = SocketAddr::new((*ip).into(), port);
                                let node_state = NodeState::new(id.clone().into(), 0, addr);
                                debug!("Adding peer: {} at {}", id, addr);
                                state.add_node(id.into(), node_state).await;
                            } else {
                                error!("Could not resolve IP address for service: {}", id);
                            }
                        }
                    }
                    ServiceEvent::ServiceRemoved(service_info, fullname) => {
                        info!("Removed service: {} ({})", service_info, fullname);
                        let id = NodeId::from(fullname.to_string());
                        state.remove_node(&id).await;
                        debug!("Removed peer: {}", id);
                    }
                    _ => {}
                }
            }
        });
        Ok(handle)
    }
}

impl std::fmt::Display for Service {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Service {{ id: {} }}", self.state.id())
    }
}
