//! BitClaw Sender - Send test P2P messages to a listener
//!
//! Usage:
//!   bitclaw-sender --target-ip 127.0.0.1 --target-port 60000 --message "Hello from sender!"
//!

use bitclaw_client::{TrackerClient, ClientConfig};
use std::net::IpAddr;

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("warn")
    ).init();

    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: bitclaw-sender --target-ip <ip> --target-port <port> --message <msg>");
        eprintln!();
        eprintln!("Options:");
        eprintln!("  --target-ip     Target listener IP (required)");
        eprintln!("  --target-port   Target listener port (required)");
        eprintln!("  --message       Message to send (default: 'Test message')");
        eprintln!("  --tracker-url   Tracker URL for registration (default: http://127.0.0.1:8080)");
        eprintln!("  --name          Sender name (default: test-sender)");
        std::process::exit(1);
    }

    let target_ip: IpAddr = get_arg_value(&args, "target-ip")
        .unwrap_or_else(|| "127.0.0.1".to_string())
        .parse()
        .expect("Invalid IP address");

    let target_port: u16 = get_arg_value(&args, "target-port")
        .unwrap_or_else(|| "0".to_string())
        .parse()
        .expect("Invalid port");

    let message = get_arg_value(&args, "message")
        .unwrap_or_else(|| "Hello from bitclaw-sender!".to_string());

    let tracker_url = get_arg_value(&args, "tracker-url")
        .unwrap_or_else(|| "http://127.0.0.1:8080".to_string());

    let name = get_arg_value(&args, "name")
        .unwrap_or_else(|| "test-sender".to_string());

    if target_port == 0 {
        eprintln!("Error: --target-port is required");
        std::process::exit(1);
    }

    eprintln!("=== BitClaw Sender ===");
    eprintln!("Target: {}:{} ", target_ip, target_port);
    eprintln!("Message: {}", message);
    eprintln!();

    // Create client
    let config = ClientConfig::lan_mode(tracker_url.clone(), name.clone());

    let client = TrackerClient::new(config)
        .await
        .expect("Failed to create client");

    eprintln!("Client ID: {}", client.client_id());
    eprintln!("Local address: {:?}", client.local_addr());
    eprintln!();

    // Connect to target
    eprintln!("Connecting to {}:{}...", target_ip, target_port);

    let peer_id = client.connect(target_ip, target_port)
        .await
        .expect("Failed to connect to target");

    eprintln!("Connected to peer: {}", peer_id);
    eprintln!();

    // Send message
    eprintln!("Sending message...");
    client.send_text_to_peer(&peer_id, &message)
        .await
        .expect("Failed to send message");

    eprintln!("Message sent successfully!");

    // Keep connection alive briefly for delivery
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Cleanup
    let _ = client.disconnect_peer(&peer_id).await;
    let _ = client.shutdown().await;

    eprintln!();
    eprintln!("Done.");
}

fn get_arg_value(args: &[String], name: &str) -> Option<String> {
    for i in 0..args.len() {
        if args[i] == name || args[i] == format!("--{}", name) {
            if i + 1 < args.len() && !args[i + 1].starts_with('-') {
                return Some(args[i + 1].clone());
            }
        }
    }
    None
}
