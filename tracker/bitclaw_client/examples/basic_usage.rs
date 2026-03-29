//! Example demonstrating the Arcadia Client API
//!
//! This example shows:
//! - Creating a client in LAN mode (no UPnP)
//! - Creating a client with UPnP enabled
//! - Listing hubs
//! - Connecting to a hub
//! - Finding agents
//! - Connecting to other clients
//! - Peer-to-peer messaging

use arcadia_client::{ClientConfig, TrackerClient, ClientError, MessageContent};
use std::net::Ipv4Addr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Example 1: LAN mode (no UPnP)
    let config = ClientConfig::lan_mode(
        "http://localhost:8000".to_string(),
        "my-lan-client".to_string(),
    );

    println!("Creating client in LAN mode...");
    let client = TrackerClient::new(config).await?;

    println!("Client ID: {}", client.client_id());
    println!("Listening on: {}", client.local_addr().unwrap());
    println!("UPnP enabled: {}", client.is_upnp_enabled());

    // Example 2: List available hubs
    println!("\nListing available hubs...");
    match client.list_hubs().await {
        Ok(hubs) => {
            for hub in &hubs {
                println!("  - {} (ID: {})", hub.name, hub.hub_id);
            }
        }
        Err(e) => {
            println!("Failed to list hubs: {}", e);
        }
    }

    // Example 3: Connect to a hub
    println!("\nConnecting to 'general' hub...");
    match client.connect_hub("general").await {
        Ok(hub) => {
            println!("Connected to hub: {} ({})", hub.name, hub.hub_id);
        }
        Err(e) => {
            println!("Failed to connect to hub: {}", e);
        }
    }

    // Example 4: Find agents in the hub
    println!("\nSearching for agents with capability 'searcher'...");
    match client.find_agent("general", "searcher").await {
        Ok(agents) => {
            for agent in &agents {
                println!("  - {} - {}", agent.name, agent.description);
                println!("    Capabilities: {:?}", agent.capabilities);
            }
        }
        Err(e) => {
            println!("Failed to find agents: {}", e);
        }
    }

    // Example 5: Connect to another client (if you know their address)
    // Uncomment and modify for actual use:
    // println!("\nConnecting to peer at 192.168.1.100:8080...");
    // match client.connect(Ipv4Addr::new(192, 168, 1, 100).into(), 8080).await {
    //     Ok(peer_id) => {
    //         println!("Connected to peer: {}", peer_id);
    //
    //         // Send a message
    //         client.send_text_to_peer(&peer_id, "Hello!").await?;
    //         println!("Sent message to peer");
    //     }
    //     Err(e) => {
    //         println!("Failed to connect to peer: {}", e);
    //     }
    // }

    // Example 6: Broadcast to all connected peers
    // client.broadcast_to_peers(MessageContent::Text("Hello everyone!".to_string())).await?;

    // Clean shutdown
    println!("\nShutting down client...");
    client.shutdown().await?;

    Ok(())
}

/// Example with UPnP enabled (for WAN/Internet accessible clients)
#[allow(dead_code)]
async fn example_with_upnp() -> Result<(), ClientError> {
    let config = ClientConfig::with_upnp(
        "http://tracker.example.com:8000".to_string(),
        "my-wan-client".to_string(),
        Some(12345), // External port (optional, defaults to same as local)
    );

    let client = TrackerClient::new(config).await?;

    println!("Client with UPnP started");
    println!("Local address: {}", client.local_addr().unwrap());
    println!("Public address: {:?}", client.public_addr());

    // The client is now accessible from the internet via the public address
    // Other clients can connect using: client.connect(public_ip, public_port)

    client.shutdown().await
}

/// Example: Creating a client with custom configuration
#[allow(dead_code)]
async fn example_custom_config() -> Result<(), ClientError> {
    use arcadia_client::UpnpConfig;

    let config = ClientConfig {
        tracker_url: "http://localhost:8000".to_string(),
        local_ip: Ipv4Addr::LOCALHOST.into(),
        local_port: 8080,
        client_name: "custom-client".to_string(),
        upnp_config: UpnpConfig {
            enabled: true,
            external_port: Some(12345),
            lease_duration: 3600, // 1 hour lease
            description: "My Arcadia Client".to_string(),
        },
    };

    let client = TrackerClient::new(config).await?;

    // Use the client...

    client.shutdown().await
}
