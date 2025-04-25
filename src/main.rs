use axum::{Json, extract::State};
use axum::{Router, routing::post};
use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};
use serde::{Deserialize, Serialize};
use std::net::Ipv4Addr;
use std::{collections::HashMap, net::SocketAddr, sync::Arc, time::Duration};
use tokio::net::TcpListener;
use tokio::task::JoinHandle;
use tokio::{sync::Mutex, time::interval};
use tracing::{error, info, trace};
use uuid::Uuid;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let id = Uuid::new_v4().to_string();
    println!("Node ID: {}", id);
    let ip = Ipv4Addr::new(192, 168, 1, 102);
    println!("Node IP: {}", ip);
    let port = 3000;
    println!("Node Port: {}", port);
    println!("Starting node service...");
    let mut mdns_service = MdnsService::new(&id, ip, port, &[]);
    println!("Starting whispering...");
    let whisperer = mdns_service.whisper().await?;
    println!("Starting listening...");
    let listener = mdns_service.listen().await?;
    println!("Starting browsing...");
    let browser = mdns_service.browse(SERVICE_TYPE).await?;

    // Keep the main thread alive
    loop {
        tokio::time::sleep(Duration::from_secs(60)).await;
    }
    println!("Stopping gossip loop...");
    whisperer.abort();
    listener.abort();
    browser.abort();
    Ok(())
}

pub struct MdnsService {
    ip: Ipv4Addr,
    port: u16,
    daemon: ServiceDaemon,
    service: ServiceInfo,
    state: AppState,
}

const SERVICE_TYPE: &str = "_whispers._tcp.local.";
const DOMAIN_NAME: &str = "local.";

impl MdnsService {
    pub fn new(id: &str, ip: Ipv4Addr, port: u16, properties: &[(String, String)]) -> Self {
        let daemon = ServiceDaemon::new().expect("Failed to create daemon");
        let id = id.to_string();
        let my_name = &id;
        let host_name = format!("{}.{}", my_name, DOMAIN_NAME);
        let service = ServiceInfo::new(
            SERVICE_TYPE,
            &my_name,
            &host_name,
            ip.to_string(),
            port,
            properties,
        )
        .expect("Failed to create service info");
        daemon
            .register(service.clone())
            .expect("Failed to register service");
        println!(
            "Advertising service: {} on {}:{}",
            service.get_fullname(),
            ip,
            port
        );
        let peers = vec![];
        let state = AppState::new(id.clone(), peers);
        Self {
            ip,
            port,
            daemon,
            service,
            state,
        }
    }

    pub async fn whisper(&mut self) -> eyre::Result<JoinHandle<()>> {
        let handle = tokio::spawn(start_gossip_loop(self.state.clone()));
        Ok(handle)
    }

    pub async fn listen(&mut self) -> eyre::Result<JoinHandle<()>> {
        let app = Router::new()
            .route("/gossip", post(gossip_handler))
            .with_state(self.state.clone());
        let listener = TcpListener::bind((self.ip, self.port)).await?;
        let addr = format!("{}:{}", self.ip, self.port);
        let handle = tokio::spawn(async move {
            println!("Listening on: {}", addr);
            axum::serve(listener, app)
                .await
                .map_err(|e| eyre::eyre!("Failed to start server: {}", e))
                .unwrap();
        });
        Ok(handle)
    }

    pub async fn browse(&mut self, service_type: &str) -> eyre::Result<JoinHandle<()>> {
        let receiver = self
            .daemon
            .browse(service_type)
            .expect("Failed to browse for services");
        let my_fullname = self.service.get_fullname().to_string();

        let handle = tokio::spawn(async move {
            while let Ok(event) = receiver.recv() {
                match event {
                    ServiceEvent::SearchStarted(service_type) => {
                        trace!("Started searching for a service type.: {}", service_type);
                    }
                    ServiceEvent::ServiceRemoved(service_type, fullname) => {
                        println!("Removed service: {:?} of name {}", service_type, fullname);
                    }
                    ServiceEvent::ServiceFound(service_type, fullname) => {
                        if fullname == my_fullname {
                            println!(
                                "Found my own service: {:?} of name {}",
                                service_type, fullname
                            );
                        } else {
                            println!("Found service: {:?} of name {}", service_type, fullname);
                        }
                    }
                    ServiceEvent::ServiceResolved(service_info) => {
                        println!("Resolved service: {:?}", service_info);
                    }
                    ServiceEvent::SearchStopped(info) => {
                        println!("Stopped searching for service: {}", info);
                    }
                }
            }
        });
        println!("Browsing for services of type: {}", service_type);
        Ok(handle)
    }
}

impl Drop for MdnsService {
    fn drop(&mut self) {
        println!("Stopping mDNS service...");
        self.daemon
            .unregister(self.service.get_fullname())
            .expect("Failed to unregister service");
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeState {
    last_seen: u64,
    load: f32,
    services: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GossipPayload {
    from: String,
    state: HashMap<String, NodeState>,
}

#[derive(Clone)]
pub struct AppState {
    pub id: String,
    pub state: Arc<Mutex<HashMap<String, NodeState>>>,
    pub peers: Vec<SocketAddr>,
}

impl AppState {
    pub fn new(id: String, peers: Vec<SocketAddr>) -> Self {
        Self {
            id,
            state: Arc::new(Mutex::new(HashMap::new())),
            peers,
        }
    }
}

pub async fn gossip_handler(
    State(app): State<AppState>,
    Json(payload): Json<GossipPayload>,
) -> &'static str {
    let mut local = app.state.lock().await;
    println!("Received gossip from: {}", payload.from);
    println!("State: {:?}", payload.state);
    println!("Local state: {:?}", local);
    println!("Peers: {:?}", app.peers);
    for (key, incoming) in payload.state {
        match local.get(&key) {
            Some(existing) => {
                if incoming.last_seen > existing.last_seen {
                    local.insert(key, incoming);
                }
            }
            None => {
                local.insert(key, incoming);
            }
        }
    }
    "ok"
}

pub async fn start_gossip_loop(app: AppState) {
    let client = reqwest::Client::new();
    let mut interval = interval(Duration::from_secs(5));
    println!("Starting gossip loop...");
    loop {
        interval.tick().await;

        let state = app.state.lock().await.clone();
        let payload = GossipPayload {
            from: app.id.clone(),
            state,
        };

        for peer in &app.peers {
            let _ = client
                .post(format!("http://{}/gossip", peer))
                .json(&payload)
                .send()
                .await;
        }
    }
}
