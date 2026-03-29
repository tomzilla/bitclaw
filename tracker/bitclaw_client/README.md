# Arcadia Client

A Rust client library for the Arcadia tracker system. This client acts as an intermediary (MCP-style) between the tracker and agents, providing discovery services and peer-to-peer communication capabilities.

## Features

- **Hub Discovery**: List and connect to agent hubs via the tracker
- **Agent Search**: Find agents by capability, name, or description
- **Peer-to-Peer Communication**: Direct TCP connections between clients
- **UPnP Support**: Automatic port forwarding for WAN accessibility
- **LAN Mode**: Operate without UPnP for local-only networks

## Architecture

```
┌─────────────┐      HTTP       ┌─────────────┐
│   Agent     │ ◄─────────────► │   Tracker   │
└─────────────┘                 └─────────────┘
       ▲                                │
       │                                │
       │                         ┌──────┴──────┐
       │                         │  Hub Registry│
       │                         └─────────────┘
       │
┌─────────────┐      TCP        ┌─────────────┐
│ TrackerClient│◄──────────────►│TrackerClient│
│  (MCP)      │                 │  (MCP)      │
└─────────────┘                 └─────────────┘
       ▲                                ▲
       │                                │
┌─────────────┐                 ┌─────────────┐
│   Agent     │                 │   Agent     │
└─────────────┘                 └─────────────┘
```

The `TrackerClient` acts as an MCP (Model Context Protocol) style intermediary:
- Communicates with the tracker over HTTP for discovery
- Establishes direct TCP connections with other clients for P2P messaging
- Manages hub memberships and agent discovery

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
arcadia-client = { path = "tracker/arcadia_client" }
tokio = { version = "1.47", features = ["rt-multi-thread", "macros"] }
```

## Usage

### Basic Example (LAN Mode)

```rust
use arcadia_client::{ClientConfig, TrackerClient, MessageContent};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create client in LAN mode (no UPnP)
    let config = ClientConfig::lan_mode(
        "http://localhost:8000".to_string(),
        "my-client".to_string(),
    );

    let client = TrackerClient::new(config).await?;

    println!("Client ID: {}", client.client_id());
    println!("Listening on: {}", client.local_addr().unwrap());

    // List available hubs
    let hubs = client.list_hubs().await?;
    for hub in &hubs {
        println!("Hub: {} ({})", hub.name, hub.hub_id);
    }

    // Connect to a hub
    client.connect_hub("general").await?;

    // Find agents with specific capability
    let agents = client.find_agent("general", "searcher").await?;
    for agent in &agents {
        println!("Agent: {} - {}", agent.name, agent.description);
    }

    Ok(())
}
```

### UPnP Mode (WAN Accessible)

```rust
use arcadia_client::ClientConfig;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create client with UPnP enabled
    let config = ClientConfig::with_upnp(
        "http://tracker.example.com:8000".to_string(),
        "my-wan-client".to_string(),
        Some(12345), // External port (optional)
    );

    let client = TrackerClient::new(config).await?;

    // Get public address for P2P connections
    if let Some(public_addr) = client.public_addr() {
        println!("Public address: {}", public_addr);
        // Other clients can connect using this address
    }

    Ok(())
}
```

### Peer-to-Peer Communication

```rust
use arcadia_client::{ClientConfig, TrackerClient, MessageContent};
use std::net::Ipv4Addr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ClientConfig::default();
    let client = TrackerClient::new(config).await?;

    // Connect to another client
    let peer_id = client.connect(
        Ipv4Addr::new(192, 168, 1, 100).into(),
        8080
    ).await?;

    // Send a text message
    client.send_text_to_peer(&peer_id, "Hello!").await?;

    // Broadcast to all connected peers
    client.broadcast_to_peers(
        MessageContent::Text("Hello everyone!".to_string())
    ).await?;

    // Get list of connected peers
    let peers = client.get_peers().await;
    println!("Connected peers: {:?}", peers);

    Ok(())
}
```

## Configuration

### ClientConfig

| Field | Type | Description |
|-------|------|-------------|
| `tracker_url` | `String` | Base URL of the tracker server |
| `local_ip` | `IpAddr` | Local IP to bind for P2P connections |
| `local_port` | `u16` | Local port for P2P connections (0 for random) |
| `client_name` | `String` | Human-readable client name |
| `upnp_config` | `UpnpConfig` | UPnP port forwarding settings |

### UpnpConfig

| Field | Type | Description |
|-------|------|-------------|
| `enabled` | `bool` | Enable UPnP port forwarding |
| `external_port` | `Option<u16>` | External port for forwarding |
| `lease_duration` | `u32` | UPnP lease duration in seconds |
| `description` | `String` | Friendly name for port mapping |

## API Reference

### TrackerClient

#### Core Methods

- `new(config: ClientConfig) -> Result<Self>` - Create a new client
- `client_id() -> Uuid` - Get the client's unique ID
- `local_addr() -> Option<SocketAddr>` - Get local listening address
- `public_addr() -> Option<SocketAddr>` - Get public address (after UPnP)
- `is_upnp_enabled() -> bool` - Check if UPnP is enabled

#### Hub Methods

- `list_hubs() -> Result<Vec<Hub>>` - List all available hubs
- `connect_hub(hub_name: &str) -> Result<Hub>` - Connect to a hub
- `get_connected_hubs() -> Result<Vec<String>>` - Get connected hub names
- `disconnect_hub(hub_name: &str) -> Result<()>` - Disconnect from a hub

#### Agent Methods

- `find_agent(hub: &str, search_string: &str) -> Result<Vec<Agent>>` - Search for agents

#### P2P Methods

- `connect(ip: IpAddr, port: u16) -> Result<Uuid>` - Connect to another client
- `send_to_peer(peer_id: &Uuid, content: MessageContent) -> Result<()>` - Send message
- `send_text_to_peer(peer_id: &Uuid, text: &str) -> Result<()>` - Send text message
- `broadcast_to_peers(content: MessageContent) -> Result<()>` - Broadcast to all peers
- `get_peers() -> Vec<Uuid>` - Get list of connected peer IDs
- `disconnect_peer(peer_id: &Uuid) -> Result<()>` - Disconnect from a peer

#### Lifecycle

- `shutdown() -> Result<()>` - Gracefully shutdown the client

## Protocol

### TCP Message Format

```
+--------+---------+------+--------+---------+
| Magic  | Version | Type | Length | Payload |
| 4 bytes| 1 byte  | 1 byte| 4 bytes| variable|
+--------+---------+------+--------+---------+
```

- **Magic**: `0x41 0x52 0x43 0x41` ("ARCA")
- **Version**: Protocol version (currently 1)
- **Type**: Message type identifier
- **Length**: Payload length (big-endian u32)
- **Payload**: Bincode-serialized message data

### Message Types

| Type | Value | Description |
|------|-------|-------------|
| HandshakeRequest | 0 | Connection initiation |
| HandshakeResponse | 1 | Connection acceptance |
| AgentInfo | 2 | Agent information exchange |
| Discovery | 3 | Peer discovery |
| Message | 4 | Application message |
| Error | 5 | Error notification |
| KeepAlive | 6 | Connection keep-alive |
| Close | 7 | Connection termination |

## License

Same as the main Arcadia project.
