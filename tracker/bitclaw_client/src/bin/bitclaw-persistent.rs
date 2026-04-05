//! BitClaw Persistent Client - Long-running client that stays connected to the tracker
//!
//! Usage:
//!   bitclaw-persistent --tracker-url http://localhost:8080 --name my-client --hub general
//!

use bitclaw_client::{TrackerClient, ClientConfig};

#[tokio::main]
async fn main() {
    // Initialize logging
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info")
    ).init();

    let args: Vec<String> = std::env::args().collect();

    let tracker_url = get_arg_value(&args, "tracker-url")
        .unwrap_or_else(|| "http://127.0.0.1:8080".to_string());
    let name = get_arg_value(&args, "name")
        .unwrap_or_else(|| "persistent-client".to_string());
    let hub = get_arg_value(&args, "hub")
        .unwrap_or_else(|| "general".to_string());
    let lan_mode = has_arg(&args, "lan-mode") || has_arg(&args, "lan");

    println!("=== BitClaw Persistent Client ===");
    println!("Tracker URL: {}", tracker_url);
    println!("Client Name: {}", name);
    println!("Hub: {}", hub);
    println!("LAN Mode: {}", lan_mode);
    println!();

    let config = if lan_mode {
        ClientConfig::lan_mode(tracker_url.clone(), name.clone())
    } else {
        ClientConfig::with_upnp(tracker_url.clone(), name.clone(), None)
    };

    let client = match TrackerClient::new(config).await {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to create client: {}", e);
            std::process::exit(1);
        }
    };

    println!("Client ID: {}", client.client_id());
    println!("Listening on: {}", client.local_addr().unwrap());
    if let Some(public_addr) = client.public_addr() {
        println!("Public address: {}", public_addr);
    }
    println!();

    // List hubs
    println!("Available hubs:");
    match client.list_hubs().await {
        Ok(hubs) => {
            for hub_info in &hubs {
                println!("  - {} (ID: {})", hub_info.name, hub_info.hub_id);
            }
        }
        Err(e) => eprintln!("Failed to list hubs: {}", e),
    }
    println!();

    // Register agent first (required before connecting to hub)
    println!("Registering agent with tracker...");
    let agent_registered = match register_agent(&client, &name).await {
        Ok(_) => {
            println!("Agent registered successfully");
            true
        }
        Err(e) => {
            eprintln!("Note: Agent registration skipped - may already be registered: {}", e);
            false
        }
    };
    println!();

    // Connect to hub
    println!("Connecting to '{}' hub...", hub);
    if agent_registered {
        match client.connect_hub(&hub).await {
            Ok(hub_info) => {
                println!("Connected to hub: {} ({})", hub_info.name, hub_info.hub_id);
            }
            Err(e) => {
                eprintln!("Failed to connect to hub: {}", e);
            }
        }
    } else {
        eprintln!("Skipping hub connection - agent not registered");
    }
    println!();

    println!("Client is now running. Press Ctrl+C to exit.");
    println!("Connected peers will be shown below.");
    println!();

    // Set up signal handlers for graceful shutdown
    let shutdown_client = client.clone();
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.expect("Failed to listen for Ctrl+C");
        println!("\nShutting down...");
        let _ = shutdown_client.shutdown().await;
        std::process::exit(0);
    });

    // Keep track of last peer count for change detection
    let mut last_peer_count = 0;

    // Main loop - periodically check for peers and messages
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;

        let peers = client.get_peers().await;
        if peers.len() != last_peer_count {
            last_peer_count = peers.len();
            println!("[{}] Connected peers: {}",
                chrono::Local::now().format("%H:%M:%S"),
                peers.len());
            for peer in &peers {
                println!("    - {}", peer);
            }
        }

        // Keep connection alive by checking connected hubs
        let hubs = client.get_connected_hubs().await;
        if hubs.is_empty() {
            println!("[{}] Reconnecting to hub...",
                chrono::Local::now().format("%H:%M:%S"));
            let _ = client.connect_hub(&hub).await;
        } else {
            log::debug!("Connected to hubs: {:?}", hubs);
        }
    }
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

fn has_arg(args: &[String], name: &str) -> bool {
    args.iter().any(|a| a == name || a == &format!("--{}", name))
}

/// Register agent with the tracker
async fn register_agent(client: &TrackerClient, name: &str) -> Result<(), String> {
    use reqwest::Client;

    // Use the tracker URL from client config (need to get it somehow)
    // For now, use the ARCADIA_TRACKER_URL env var or default
    let tracker_url = std::env::var("ARCADIA_TRACKER_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:8080".to_string());

    let register_url = format!("{}/api/v1/agents", tracker_url);

    let local_addr = client.local_addr()
        .map(|a| a.to_string())
        .unwrap_or_else(|| "127.0.0.1:0".to_string());

    let (ip, port) = parse_addr(&local_addr).unwrap_or_else(|| ("127.0.0.1".to_string(), 0));

    let request_body = serde_json::json!({
        "agent_id": Some(client.client_id().to_string()),
        "name": name,
        "description": format!("Persistent client: {}", name),
        "capabilities": vec!["general"],
        "ip_address": ip,
        "port": port,
        "endpoint": None::<String>,
        "hubs": None::<Vec<String>>,
        "metadata": None::<serde_json::Value>
    });

    let http_client = Client::new();
    let response = http_client
        .post(&register_url)
        .json(&request_body)
        .send()
        .await
        .map_err(|e| format!("HTTP error: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("Status: {} - {}", status, body));
    }

    Ok(())
}

/// Parse address string into (ip, port)
fn parse_addr(addr: &str) -> Option<(String, u16)> {
    let parts: Vec<&str> = addr.split(':').collect();
    if parts.len() == 2 {
        let port = parts[1].parse().ok()?;
        Some((parts[0].to_string(), port))
    } else {
        None
    }
}
