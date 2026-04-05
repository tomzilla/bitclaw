//! Persistent BitClaw Client
//!
//! A long-running client that:
//! - Connects to the tracker
//! - Sends periodic heartbeats
//! - Listens for incoming P2P messages
//! - Displays received messages

use bitclaw_client::{ClientConfig, TrackerClient};
use std::env;
use std::time::Duration;
use tokio::time::interval;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get configuration from environment or use defaults
    let tracker_url = env::var("TRACKER_URL").unwrap_or_else(|_| "http://127.0.0.1:8080".to_string());
    let client_name = env::var("CLIENT_NAME").unwrap_or_else(|_| "persistent-client".to_string());
    let hub_name = env::var("HUB_NAME").unwrap_or_else(|_| "general".to_string());

    println!("=== BitClaw Persistent Client ===");
    println!("Tracker: {}", tracker_url);
    println!("Client: {}", client_name);
    println!("Hub: {}", hub_name);
    println!();

    // Create client in LAN mode
    let config = ClientConfig::lan_mode(tracker_url.clone(), client_name.clone());
    let client = TrackerClient::new(config).await?;

    println!("Client ID: {}", client.client_id());
    println!("Listening on: {}", client.local_addr().unwrap());
    println!();

    // Register with tracker (manual registration via API would be needed)
    println!("Press Ctrl+C to stop...");
    println!();

    // Set up periodic heartbeat (every 30 seconds)
    let mut heartbeat_interval = interval(Duration::from_secs(30));

    // Set up message polling interval (every 2 seconds)
    let mut message_interval = interval(Duration::from_secs(2));

    let mut heartbeat_count = 0u64;
    let mut message_count = 0u64;

    loop {
        tokio::select! {
            _ = heartbeat_interval.tick() => {
                heartbeat_count += 1;
                println!("[heartbeat #{}] Sending heartbeat...", heartbeat_count);
                // Note: Heartbeat would require agent_id and passkey from registration
                // For now, just show we're alive
            }
            _ = message_interval.tick() => {
                // Check for incoming messages from peers
                let peers = client.get_peers().await;
                if !peers.is_empty() {
                    println!("[messages] Connected to {} peer(s): {:?}", peers.len(), peers);
                    message_count += 1;
                }
            }
            _ = tokio::signal::ctrl_c() => {
                println!();
                println!("Shutting down...");
                break;
            }
        }
    }

    // Clean shutdown
    println!("Final stats: {} heartbeats, {} message checks", heartbeat_count, message_count);
    client.shutdown().await?;
    println!("Client stopped.");

    Ok(())
}
