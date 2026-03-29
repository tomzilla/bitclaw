//! Main tracker client implementation
//!
//! Provides the high-level API for:
//! - Listing and connecting to hubs
//! - Finding agents
//! - Peer-to-peer communication with other clients

use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::error::{ClientError, Result};
use crate::tcp::{ClientTcpListener, ClientConnection};
use crate::protocol::{ClientMessage, MessageContent};
use crate::upnp::UpnpConfig;
use serde::Deserialize;

/// Hub information received from the tracker
#[derive(Debug, Clone, Deserialize)]
pub struct Hub {
    pub hub_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub max_agents: Option<i32>,
    pub is_public: bool,
}

/// Agent information received from the tracker
#[derive(Debug, Clone, Deserialize)]
pub struct Agent {
    pub agent_id: String,
    pub name: String,
    pub description: String,
    pub capabilities: Vec<String>,
    pub endpoint: Option<String>,
    pub status: String,
    pub ip_address: Option<IpAddr>,
    pub port: Option<u16>,
}

/// Configuration for the tracker client
#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// Tracker base URL (e.g., "http://localhost:8000")
    pub tracker_url: String,
    /// Local IP address to bind for P2P connections
    pub local_ip: IpAddr,
    /// Local port to bind for P2P connections (0 for random)
    pub local_port: u16,
    /// Client name for identification
    pub client_name: String,
    /// UPnP configuration (set enabled=false for LAN mode)
    pub upnp_config: UpnpConfig,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            tracker_url: "http://localhost:8000".to_string(),
            local_ip: IpAddr::V4(std::net::Ipv4Addr::new(0, 0, 0, 0)),
            local_port: 0,
            client_name: "unnamed-client".to_string(),
            upnp_config: UpnpConfig::default(),
        }
    }
}

impl ClientConfig {
    /// Create a new config for LAN-only operation (no UPnP)
    pub fn lan_mode(tracker_url: String, client_name: String) -> Self {
        Self {
            tracker_url,
            client_name,
            upnp_config: UpnpConfig {
                enabled: false,
                ..Default::default()
            },
            ..Default::default()
        }
    }

    /// Create a new config with UPnP enabled
    pub fn with_upnp(tracker_url: String, client_name: String, external_port: Option<u16>) -> Self {
        Self {
            tracker_url,
            client_name,
            upnp_config: UpnpConfig {
                enabled: true,
                external_port,
                ..Default::default()
            },
            ..Default::default()
        }
    }
}

/// Connected hub state
#[derive(Debug, Clone)]
pub struct ConnectedHub {
    pub hub: Hub,
    pub connected_at: chrono::DateTime<chrono::Utc>,
}

/// Tracker Client - Main entry point for client operations
///
/// This client acts as an intermediary (MCP-style) between the tracker and agents.
/// It provides discovery services and facilitates peer-to-peer communication.
///
/// # Example
/// ```no_run
/// use arcadia_client::{TrackerClient, ClientConfig};
/// use std::net::Ipv4Addr;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let config = ClientConfig {
///         tracker_url: "http://localhost:8000".to_string(),
///         local_ip: Ipv4Addr::LOCALHOST.into(),
///         local_port: 0,
///         client_name: "my-client".to_string(),
///     };
///
///     let client = TrackerClient::new(config).await?;
///
///     // List available hubs
///     let hubs = client.list_hubs().await?;
///     println!("Available hubs: {:?}", hubs);
///
///     // Connect to a hub
///     client.connect_hub("my-hub").await?;
///
///     // Find agents in the hub
///     let agents = client.find_agent("my-hub", "searcher").await?;
///     println!("Found agents: {:?}", agents);
///
///     Ok(())
/// }
/// ```
pub struct TrackerClient {
    client_id: Uuid,
    config: ClientConfig,
    http_client: reqwest::Client,
    hubs: Arc<RwLock<HashMap<String, ConnectedHub>>>,
    peers: Arc<RwLock<HashMap<Uuid, ClientConnection>>>,
    tcp_listener: Option<ClientTcpListener>,
    /// Public address for P2P connections (after UPnP)
    public_addr: Option<SocketAddr>,
    /// Whether UPnP is enabled
    upnp_enabled: bool,
}

impl TrackerClient {
    /// Create a new tracker client
    pub async fn new(config: ClientConfig) -> Result<Self> {
        let client_id = Uuid::new_v4();
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()?;

        // Start TCP listener for P2P connections
        let tcp_listener = ClientTcpListener::bind(
            client_id,
            config.local_ip,
            config.local_port,
        ).await?;

        let local_addr = tcp_listener.local_addr();

        // Set up UPnP port forwarding if enabled
        let mut public_addr = None;
        let upnp_enabled = config.upnp_config.enabled;

        if upnp_enabled {
            match crate::upnp::setup_port_forwarding(local_addr, &config.upnp_config).await {
                Ok(mapping) => {
                    log::info!(
                        "Port mapping established: internal={} external={} public={}",
                        local_addr,
                        mapping.external_addr,
                        mapping.public_ip
                    );
                    public_addr = Some(mapping.external_addr);
                }
                Err(e) => {
                    log::warn!("UPnP setup failed: {}. Operating in LAN mode.", e);
                }
            }
        } else {
            log::info!("UPnP disabled - operating in LAN mode");
        }

        log::info!(
            "TrackerClient {} started, listening on {}",
            client_id,
            local_addr
        );

        Ok(Self {
            client_id,
            config,
            http_client,
            hubs: Arc::new(RwLock::new(HashMap::new())),
            peers: Arc::new(RwLock::new(HashMap::new())),
            tcp_listener: Some(tcp_listener),
            public_addr,
            upnp_enabled,
        })
    }
    /// Get the client's unique ID
    pub fn client_id(&self) -> Uuid {
        self.client_id
    }

    /// Get the local address for P2P connections
    pub fn local_addr(&self) -> Option<SocketAddr> {
        self.tcp_listener.as_ref().map(|l| l.local_addr())
    }

    /// Get the public address for P2P connections (after UPnP forwarding)
    /// Returns None if UPnP is not enabled or failed
    pub fn public_addr(&self) -> Option<SocketAddr> {
        self.public_addr
    }

    /// Check if UPnP is enabled
    pub fn is_upnp_enabled(&self) -> bool {
        self.upnp_enabled
    }

    /// List all available hubs from the tracker
    ///
    /// This queries the tracker's hub registry and returns all public hubs
    /// that the client can potentially join.
    pub async fn list_hubs(&self) -> Result<Vec<Hub>> {
        let url = format!("{}/api/v1/hubs", self.config.tracker_url);

        let response = self.http_client
            .get(&url)
            .send()
            .await
            .map_err(|e| ClientError::Http(e))?;

        if !response.status().is_success() {
            return Err(ClientError::Connection(format!(
                "Failed to list hubs: {}",
                response.status()
            )));
        }

        // Response is {"hubs": [...]} not direct array
        #[derive(serde::Deserialize)]
        struct HubResponse {
            hubs: Vec<Hub>,
        }

        let hub_response: HubResponse = response.json().await
            .map_err(|e| ClientError::Http(e))?;

        Ok(hub_response.hubs)
    }

    /// Connect to a hub by name
    ///
    /// This registers the client with the specified hub and enables
    /// discovery of other agents in the same hub.
    ///
    /// Returns the hub information if successful.
    pub async fn connect_hub(&self, hub_name: &str) -> Result<Hub> {
        // First, try to find the hub
        let hubs = self.list_hubs().await?;
        let hub = hubs.into_iter()
            .find(|h| h.name.eq_ignore_ascii_case(hub_name))
            .ok_or_else(|| ClientError::HubNotFound(hub_name.to_string()))?;

        // Register connection to hub with tracker
        let url = format!("{}/api/v1/hubs/{}/connect", self.config.tracker_url, hub.hub_id);

        let response = self.http_client
            .post(&url)
            .json(&serde_json::json!({
                "client_id": self.client_id.to_string(),
                "client_name": self.config.client_name,
                "address": self.public_addr.or(self.local_addr()).map(|a| a.to_string()),
            }))
            .send()
            .await
            .map_err(|e| ClientError::Http(e))?;

        if !response.status().is_success() {
            return Err(ClientError::Connection(format!(
                "Failed to connect to hub: {}",
                response.status()
            )));
        }

        // Store connected hub
        let connected_hub = ConnectedHub {
            hub: hub.clone(),
            connected_at: chrono::Utc::now(),
        };

        self.hubs.write().await.insert(hub.name.clone(), connected_hub);

        log::info!("Connected to hub: {}", hub.name);

        Ok(hub)
    }

    /// Find agents matching a search string within a hub
    ///
    /// Searches for agents by name, description, or capabilities.
    /// The search is scoped to the specified hub if provided.
    ///
    /// # Arguments
    /// * `hub` - Hub name to search within (use "*" for all hubs)
    /// * `search_string` - Search query (matches name, description, capabilities)
    pub async fn find_agent(&self, hub: &str, search_string: &str) -> Result<Vec<Agent>> {
        let mut url = format!("{}/api/v1/agents/search?q={}", self.config.tracker_url, urlencoding::encode(search_string));

        // Add hub filter if not wildcard
        if hub != "*" {
            // First check if we're connected to this hub
            let hubs = self.hubs.read().await;
            if let Some(hub_info) = hubs.get(hub) {
                url.push_str(&format!("&hub={}", hub_info.hub.hub_id));
            }
        }

        let response = self.http_client
            .get(&url)
            .send()
            .await
            .map_err(|e| ClientError::Http(e))?;

        if !response.status().is_success() {
            return Err(ClientError::Connection(format!(
                "Failed to find agents: {}",
                response.status()
            )));
        }

        let agents: Vec<Agent> = response.json().await
            .map_err(|e| ClientError::Http(e))?;

        Ok(agents)
    }

    /// Connect to another client by IP address and port
    ///
    /// Establishes a direct TCP connection to another client for
    /// peer-to-peer communication.
    ///
    /// # Arguments
    /// * `other_client_ip` - IP address of the other client
    /// * `other_client_port` - TCP port of the other client
    pub async fn connect(&self, other_client_ip: IpAddr, other_client_port: u16) -> Result<Uuid> {
        let connection = crate::tcp::connect_to_client(
            self.client_id,
            other_client_ip,
            other_client_port,
        ).await?;

        let peer_id = connection.peer_id();

        // Store the connection
        self.peers.write().await.insert(peer_id, connection);

        log::info!("Connected to peer {} at {}:{}", peer_id, other_client_ip, other_client_port);

        Ok(peer_id)
    }

    /// Send a message to a connected peer
    pub async fn send_to_peer(&self, peer_id: &Uuid, content: MessageContent) -> Result<()> {
        let peers = self.peers.read().await;
        let connection = peers.get(peer_id)
            .ok_or_else(|| ClientError::Connection("Peer not found".to_string()))?;

        // Log outgoing message
        match &content {
            MessageContent::Text(text) => {
                log::info!(">> Client {} SENDING to {}: Text: {}", self.client_id, peer_id, text);
            }
            MessageContent::Json(json) => {
                log::info!(">> Client {} SENDING to {}: Json: {}", self.client_id, peer_id, json);
            }
            MessageContent::Binary(data) => {
                log::info!(">> Client {} SENDING to {}: Binary ({} bytes)", self.client_id, peer_id, data.len());
            }
        }

        let message = ClientMessage {
            from: self.client_id,
            to: Some(*peer_id),
            content,
        };

        let serialized = bincode::serialize(&message)?;
        connection.send(&serialized).await?;

        Ok(())
    }

    /// Send a text message to a connected peer
    pub async fn send_text_to_peer(&self, peer_id: &Uuid, text: &str) -> Result<()> {
        self.send_to_peer(peer_id, MessageContent::Text(text.to_string())).await
    }

    /// Broadcast a message to all connected peers
    pub async fn broadcast_to_peers(&self, content: MessageContent) -> Result<()> {
        let peers = self.peers.read().await;

        // Log outgoing broadcast
        match &content {
            MessageContent::Text(text) => {
                log::info!(">> Client {} BROADCASTING: Text: {}", self.client_id, text);
            }
            MessageContent::Json(json) => {
                log::info!(">> Client {} BROADCASTING: Json: {}", self.client_id, json);
            }
            MessageContent::Binary(data) => {
                log::info!(">> Client {} BROADCASTING: Binary ({} bytes)", self.client_id, data.len());
            }
        }

        let message = ClientMessage {
            from: self.client_id,
            to: None,
            content,
        };

        let serialized = bincode::serialize(&message)?;

        for connection in peers.values() {
            let _ = connection.send(&serialized).await;
        }

        Ok(())
    }

    /// Get list of connected peer IDs
    pub async fn get_peers(&self) -> Vec<Uuid> {
        self.peers.read().await.keys().copied().collect()
    }

    /// Get list of connected hubs
    pub async fn get_connected_hubs(&self) -> Vec<String> {
        self.hubs.read().await.keys().cloned().collect()
    }

    /// Disconnect from a hub
    pub async fn disconnect_hub(&self, hub_name: &str) -> Result<()> {
        let mut hubs = self.hubs.write().await;
        if let Some(hub) = hubs.remove(hub_name) {
            // Notify tracker of disconnection
            let url = format!("{}/api/v1/hubs/{}/disconnect", self.config.tracker_url, hub.hub.hub_id);
            let _ = self.http_client
                .post(&url)
                .json(&serde_json::json!({ "client_id": self.client_id.to_string() }))
                .send()
                .await;

            log::info!("Disconnected from hub: {}", hub_name);
        }
        Ok(())
    }

    /// Disconnect from a peer
    pub async fn disconnect_peer(&self, peer_id: &Uuid) -> Result<()> {
        let mut peers = self.peers.write().await;
        peers.remove(peer_id);
        log::info!("Disconnected from peer: {}", peer_id);
        Ok(())
    }

    /// Shutdown the client and close all connections
    pub async fn shutdown(self) -> Result<()> {
        // Close TCP listener
        if let Some(listener) = self.tcp_listener {
            let _ = listener.shutdown().await;
        }

        // Clear all connections
        self.peers.write().await.clear();
        self.hubs.write().await.clear();

        log::info!("Client {} shutdown complete", self.client_id);

        Ok(())
    }
}

// Required for urlencoding
mod urlencoding {
    pub fn encode(s: &str) -> String {
        url_escape::encode_query(s).to_string()
    }
}
