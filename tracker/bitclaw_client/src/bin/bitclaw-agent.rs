//! BitClaw Client CLI - Command-line interface for tracker operations
//!
//! Usage:
//!   bitclaw-agent list-hubs --tracker-url http://localhost:8000
//!   bitclaw-agent register --tracker-url http://localhost:8000 --name my-agent --hub my-hub
//!   bitclaw-agent find-agent --tracker-url http://localhost:8000 --query searcher
//!   bitclaw-agent listen --tracker-url http://localhost:8000 --name my-agent
//!

use bitclaw_client::{TrackerClient, ClientConfig, ClientMessage, MessageContent, MessageHandler};
use std::sync::Arc;
use tokio::sync::mpsc;
use uuid::Uuid;

#[derive(Debug, serde::Serialize)]
struct Output<T: serde::Serialize> {
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

fn json_output<T: serde::Serialize>(data: T) -> String {
    serde_json::to_string_pretty(&Output {
        success: true,
        data: Some(data),
        error: None,
    }).unwrap()
}

fn json_error(message: &str) -> String {
    serde_json::to_string_pretty(&Output::<()> {
        success: false,
        data: None,
        error: Some(message.to_string()),
    }).unwrap()
}

fn print_json(json: &str) {
    println!("{}", json);
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

#[tokio::main]
async fn main() {
    // Initialize logging - only errors go to stderr, stdout is clean JSON
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("warn")
    ).init();

    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        print_json(&json_error("Usage: bitclaw-agent <command> [options]"));
        std::process::exit(1);
    }

    let command = &args[1];
    let rest = &args[2..];

    let result = match command.as_str() {
        "list-hubs" => cmd_list_hubs(rest).await,
        "register" => cmd_register(rest).await,
        "find-agent" => cmd_find_agent(rest).await,
        "listen" => cmd_listen(rest).await,
        "help" | "--help" | "-h" => {
            print_help();
            return;
        }
        _ => Err(format!("Unknown command: {}", command)),
    };

    match result {
        Ok(json) => print_json(&json),
        Err(e) => {
            print_json(&json_error(&e));
            std::process::exit(1);
        }
    }
}

fn print_help() {
    println!(r#"
BitClaw Agent CLI - P2P coordination via tracker

Usage: bitclaw-agent <command> [options]

Commands:
  list-hubs       List available tracker hubs
  register        Register as an agent with the tracker
  find-agent      Find agents by search query
  listen          Start persistent listener for incoming messages (JSONL output)
  help            Show this help message

Options:
  --tracker-url <url>     Tracker base URL (default: http://localhost:8000)
  --name <name>           Agent name for registration
  --description <desc>    Agent description
  --hub <name>            Hub name to join (optional)
  --query <string>        Search query for finding agents
  --lan-mode              Use LAN mode (no UPnP port forwarding)
  --port <port>           Local port for P2P connections (listen mode)

Examples:
  bitclaw-agent list-hubs --tracker-url http://localhost:8000
  bitclaw-agent register --tracker-url http://localhost:8000 --name my-agent --hub code-generation
  bitclaw-agent find-agent --tracker-url http://localhost:8000 --query "code review"
  bitclaw-agent listen --tracker-url http://localhost:8000 --name my-agent --hub general
"#);
}

async fn cmd_list_hubs(_args: &[String]) -> Result<String, String> {
    let tracker_url = get_arg_value(_args, "tracker-url")
        .unwrap_or_else(|| "http://localhost:8000".to_string());

    let config = ClientConfig::lan_mode(tracker_url.clone(), "cli-client".to_string());

    let client = TrackerClient::new(config)
        .await
        .map_err(|e| format!("Failed to create client: {}", e))?;

    let hubs = client.list_hubs()
        .await
        .map_err(|e| format!("Failed to list hubs: {}", e))?;

    #[derive(serde::Serialize)]
    struct HubList {
        hubs: Vec<HubInfo>,
    }

    #[derive(serde::Serialize)]
    struct HubInfo {
        hub_id: String,
        name: String,
        description: Option<String>,
        is_public: bool,
    }

    let hub_list = HubList {
        hubs: hubs.iter().map(|h| HubInfo {
            hub_id: h.hub_id.to_string(),
            name: h.name.clone(),
            description: h.description.clone(),
            is_public: h.is_public,
        }).collect(),
    };

    Ok(json_output(hub_list))
}

async fn cmd_register(_args: &[String]) -> Result<String, String> {
    let tracker_url = get_arg_value(_args, "tracker-url")
        .unwrap_or_else(|| "http://localhost:8000".to_string());
    let name = get_arg_value(_args, "name")
        .ok_or("Missing required argument: --name")?;
    let description = get_arg_value(_args, "description")
        .unwrap_or_else(|| format!("Auto-registered agent: {}", name));
    let hub_name = get_arg_value(_args, "hub");
    let lan_mode = has_arg(_args, "lan-mode");

    let config = if lan_mode {
        ClientConfig::lan_mode(tracker_url.clone(), name.clone())
    } else {
        ClientConfig::with_upnp(tracker_url.clone(), name.clone(), None)
    };

    let client = TrackerClient::new(config)
        .await
        .map_err(|e| format!("Failed to create client: {}", e))?;

    let local_addr = client.local_addr()
        .map(|a| a.to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let public_addr = client.public_addr()
        .map(|a| a.to_string())
        .unwrap_or_else(|| "N/A (LAN mode or UPnP failed)".to_string());

    #[derive(serde::Serialize)]
    struct RegisterResult {
        client_id: String,
        agent_name: String,
        description: String,
        local_address: String,
        public_address: String,
        upnp_enabled: bool,
        hub_joined: Option<String>,
    }

    let result = RegisterResult {
        client_id: client.client_id().to_string(),
        agent_name: name.clone(),
        description: description.clone(),
        local_address: local_addr,
        public_address: public_addr,
        upnp_enabled: client.is_upnp_enabled(),
        hub_joined: hub_name.clone(),
    };

    // Note: Actual agent registration with the tracker would require
    // a POST to /api/v1/agents endpoint, which is not yet implemented here
    // This returns the client info that would be used for registration

    drop(client);

    Ok(json_output(result))
}

async fn cmd_find_agent(_args: &[String]) -> Result<String, String> {
    let tracker_url = get_arg_value(_args, "tracker-url")
        .unwrap_or_else(|| "http://localhost:8000".to_string());
    let hub = get_arg_value(_args, "hub")
        .unwrap_or_else(|| "*".to_string());
    let query = get_arg_value(_args, "query")
        .ok_or("Missing required argument: --query")?;

    let config = ClientConfig::lan_mode(tracker_url.clone(), "cli-client".to_string());

    let client = TrackerClient::new(config)
        .await
        .map_err(|e| format!("Failed to create client: {}", e))?;

    let agents = client.find_agent(&hub, &query)
        .await
        .map_err(|e| format!("Failed to find agents: {}", e))?;

    #[derive(serde::Serialize)]
    struct AgentList {
        hub: String,
        query: String,
        agents: Vec<AgentInfo>,
    }

    #[derive(serde::Serialize)]
    struct AgentInfo {
        agent_id: String,
        name: String,
        description: String,
        status: String,
        ip_address: Option<String>,
        port: Option<u16>,
        capabilities: Vec<String>,
    }

    let agent_list = AgentList {
        hub,
        query,
        agents: agents.iter().map(|a| AgentInfo {
            agent_id: a.agent_id.clone(),
            name: a.name.clone(),
            description: a.description.clone(),
            status: a.status.clone(),
            ip_address: a.ip_address.as_ref().map(|ip| ip.to_string()),
            port: a.port,
            capabilities: a.capabilities.clone(),
        }).collect(),
    };

    Ok(json_output(agent_list))
}

async fn cmd_listen(_args: &[String]) -> Result<String, String> {
    let tracker_url = get_arg_value(_args, "tracker-url")
        .unwrap_or_else(|| "http://127.0.0.1:8080".to_string());
    let name = get_arg_value(_args, "name")
        .unwrap_or_else(|| "listen-client".to_string());
    let hub = get_arg_value(_args, "hub")
        .unwrap_or_else(|| "general".to_string());
    let _port: u16 = get_arg_value(_args, "port")
        .unwrap_or_else(|| "0".to_string())
        .parse()
        .unwrap_or(0);
    let lan_mode = has_arg(_args, "lan-mode") || has_arg(_args, "lan");

    eprintln!("=== BitClaw Listen Mode ===");
    eprintln!("Tracker URL: {}", tracker_url);
    eprintln!("Client Name: {}", name);
    eprintln!("Hub: {}", hub);
    eprintln!("LAN Mode: {}", lan_mode);
    eprintln!();
    eprintln!("Listening for incoming messages. Output is JSONL format:");
    eprintln!("  {{\"type\": \"message\", \"from\": \"<uuid>\", \"content\": {{...}}}}");
    eprintln!();
    eprintln!("Press Ctrl+C to stop.");
    eprintln!();

    let config = if lan_mode {
        ClientConfig::lan_mode(tracker_url.clone(), name.clone())
    } else {
        ClientConfig::with_upnp(tracker_url.clone(), name.clone(), None)
    };

    // Create a channel for receiving messages
    let (msg_tx, mut msg_rx) = mpsc::channel::<(Uuid, ClientMessage)>(100);

    // Create message handler that sends to channel
    let handler: MessageHandler = Arc::new(move |from, msg| {
        // Use try_send to avoid blocking in async context
        let _ = msg_tx.try_send((from, msg));
    });

    // Create client with message handler
    let client = TrackerClient::new_with_handler(config, Some(handler))
        .await
        .map_err(|e| format!("Failed to create client: {}", e))?;

    eprintln!("Client ID: {}", client.client_id());
    eprintln!("Listening on: {}", client.local_addr().unwrap());
    if let Some(public_addr) = client.public_addr() {
        eprintln!("Public address: {}", public_addr);
    }
    eprintln!();

    // Connect to hub (optional - P2P listening works without hub connection)
    eprintln!("Note: Hub connection is optional for P2P listening.");
    eprintln!("For direct P2P messaging, no hub connection is needed.");
    eprintln!();
    eprintln!("--- INCOMING MESSAGES ---");

    // Set up signal handler for graceful shutdown
    let shutdown_client = client.clone();
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.expect("Failed to listen for Ctrl+C");
        eprintln!("\nShutting down...");
        let _ = shutdown_client.shutdown().await;
        std::process::exit(0);
    });

    // Main message loop - output messages as JSONL
    while let Some((from, msg)) = msg_rx.recv().await {
        #[derive(serde::Serialize)]
        struct IncomingMessage {
            #[serde(rename = "type")]
            msg_type: &'static str,
            from: String,
            content: MessageContent,
            timestamp: String,
        }

        let output = IncomingMessage {
            msg_type: "message",
            from: from.to_string(),
            content: msg.content,
            timestamp: chrono::Utc::now().to_rfc3339(),
        };

        // Output as single-line JSON (JSONL format)
        let json_line = serde_json::to_string(&output).map_err(|e| e.to_string())?;
        println!("{}", json_line);
        // Flush stdout to ensure immediate delivery
        use std::io::Write;
        let _ = std::io::stdout().flush();
    }

    Ok("Listener stopped".to_string())
}
