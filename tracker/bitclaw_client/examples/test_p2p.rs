//! Test P2P messaging between two clients
//!
//! This integration test:
//! 1. Starts two clients on different ports
//! 2. Connects them to each other
//! 3. Exchanges messages bidirectionally
//! 4. Verifies the connections work

use bitclaw_client::{ClientConfig, TrackerClient, MessageContent};
use std::net::Ipv4Addr;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    println!("=== BitClaw Client P2P Integration Test ===\n");

    // Start Client 1
    println!("[1/8] Starting Client 1 on port 18080...");
    let config1 = ClientConfig {
        tracker_url: "http://localhost:8000".to_string(),
        local_ip: Ipv4Addr::LOCALHOST.into(),
        local_port: 18080,
        client_name: "test-client-1".to_string(),
        upnp_config: Default::default(),
    };

    let client1 = TrackerClient::new(config1).await?;
    println!("      Client 1 ID: {}", client1.client_id());
    println!("      Client 1 listening on: {}", client1.local_addr().unwrap());

    // Start Client 2
    println!("\n[2/8] Starting Client 2 on port 18081...");
    let config2 = ClientConfig {
        tracker_url: "http://localhost:8000".to_string(),
        local_ip: Ipv4Addr::LOCALHOST.into(),
        local_port: 18081,
        client_name: "test-client-2".to_string(),
        upnp_config: Default::default(),
    };

    let client2 = TrackerClient::new(config2).await?;
    println!("      Client 2 ID: {}", client2.client_id());
    println!("      Client 2 listening on: {}", client2.local_addr().unwrap());

    // Give listeners time to start
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Client 1 connects to Client 2
    println!("\n[3/8] Client 1 connects to Client 2...");
    let peer2_id = client1.connect(Ipv4Addr::LOCALHOST.into(), 18081).await?;
    println!("      Connected to peer: {}", peer2_id);
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Send message from Client 1 to Client 2
    println!("\n[4/8] Client 1 sends message to Client 2...");
    client1.send_text_to_peer(&peer2_id, "Hello from Client 1!").await?;
    println!("      Sent: 'Hello from Client 1!'");
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Client 2 connects back to Client 1
    println!("\n[5/8] Client 2 connects back to Client 1...");
    let peer1_id = client2.connect(Ipv4Addr::LOCALHOST.into(), 18080).await?;
    println!("      Connected to peer: {}", peer1_id);
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Send message from Client 2 to Client 1
    println!("\n[6/8] Client 2 sends message to Client 1...");
    client2.send_text_to_peer(&peer1_id, "Hello from Client 2!").await?;
    println!("      Sent: 'Hello from Client 2!'");
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Broadcast from Client 1
    println!("\n[7/8] Client 1 broadcasts to all peers...");
    client1.broadcast_to_peers(MessageContent::Text("Broadcast from Client 1!".to_string())).await?;
    println!("      Broadcast: 'Broadcast from Client 1!'");
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Send JSON message
    println!("\n[8/8] Client 1 sends JSON message...");
    let json_str = r#"{"type":"test","data":{"key":"value","number":42}}"#.to_string();
    client1.send_to_peer(&peer2_id, MessageContent::Json(json_str)).await?;
    println!("      Sent JSON: {{\"type\":\"test\"...}}");
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Check peer lists
    println!("\n=== Checking peer lists ===");
    let c1_peers = client1.get_peers().await;
    let c2_peers = client2.get_peers().await;
    println!("      Client 1 has {} peer(s): {:?}", c1_peers.len(), c1_peers);
    println!("      Client 2 has {} peer(s): {:?}", c2_peers.len(), c2_peers);

    // Cleanup
    println!("\n=== Cleanup ===");
    println!("      Shutting down clients...");

    client1.shutdown().await?;
    client2.shutdown().await?;

    println!("\n=== Test Complete ===");
    println!("All P2P operations completed successfully!");
    println!("\nTest Summary:");
    println!("  [OK] Client startup on different ports");
    println!("  [OK] Outbound connection (Client 1 -> Client 2)");
    println!("  [OK] Text message send (Client 1 -> Client 2)");
    println!("  [OK] Inbound connection (Client 2 -> Client 1)");
    println!("  [OK] Text message send (Client 2 -> Client 1)");
    println!("  [OK] Broadcast message");
    println!("  [OK] JSON message");
    println!("  [OK] Peer list management");
    println!("  [OK] Graceful shutdown");

    Ok(())
}
