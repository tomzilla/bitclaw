use actix_web::{web, web::Data, HttpRequest, HttpResponse};
use bitclaw_shared::tracker::models::agent::{Agent, AgentId, AgentStatus};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::Tracker;

/// Request body for agent registration
#[derive(Debug, Deserialize, ToSchema)]
pub struct AgentRegisterRequest {
    /// Optional agent ID (generated if not provided)
    pub agent_id: Option<Uuid>,
    /// Human-readable name
    pub name: String,
    /// Description of agent's purpose
    pub description: String,
    /// List of capability tags
    pub capabilities: Vec<String>,
    /// Optional endpoint URL for direct communication
    pub endpoint: Option<String>,
    /// Optional hub names to join
    pub hubs: Option<Vec<String>>,
    /// Optional metadata
    pub metadata: Option<serde_json::Value>,
    /// Optional IP address for P2P connections (uses request peer IP if not provided)
    pub ip_address: Option<String>,
    /// Optional port for P2P connections (defaults to 8080 if not provided)
    pub port: Option<u16>,
}

/// Response for agent registration
#[derive(Debug, Serialize, ToSchema)]
pub struct AgentRegisterResponse {
    pub status: String,
    pub agent_id: String,
    pub agent_passkey: String,  // Passkey for authentication
    pub interval: u64,
    /// Other agents that might be relevant
    pub discovered_agents: Vec<DiscoveredAgent>,
}

/// Discovered agent info
#[derive(Debug, Serialize, ToSchema)]
pub struct DiscoveredAgent {
    pub agent_id: String,
    pub name: String,
    pub description: String,
    pub capabilities: Vec<String>,
    pub endpoint: Option<String>,
    pub status: String,
}

#[utoipa::path(
    post,
    operation_id = "register_agent",
    tag = "Agents",
    path = "/api/v1/agents",
    request_body = AgentRegisterRequest,
    responses(
        (status = 200, description = "Agent registered successfully", body = AgentRegisterResponse),
        (status = 400, description = "Invalid request"),
        (status = 500, description = "Internal server error"),
    )
)]
pub async fn register(
    arc: Data<Tracker>,
    req: HttpRequest,
    body: web::Json<AgentRegisterRequest>,
) -> HttpResponse {
    let agent_id = match body.agent_id {
        Some(uuid) => AgentId(uuid),
        None => AgentId::new(),
    };

    let now = Utc::now();
    // Use IP from request body if provided, otherwise use peer address
    let client_ip = body.ip_address.as_ref()
        .and_then(|ip| ip.parse().ok())
        .unwrap_or_else(|| req
            .peer_addr()
            .map(|addr| addr.ip())
            .unwrap_or_else(|| std::net::IpAddr::from([127, 0, 0, 1])));

    // Use port from request body if provided, otherwise default to 8080
    let client_port = body.port.unwrap_or(8080);

    // Generate a passkey for the agent
    let passkey: [u8; 32] = rand::random();
    let passkey_hex = passkey.iter().map(|b| format!("{:02x}", b)).collect::<String>();

    // Find hub IDs from hub names
    let mut hub_ids: Vec<Uuid> = Vec::new();
    if let Some(ref hub_names) = body.hubs {
        let hubs_guard = arc.hubs.read();
        for hub_name in hub_names {
            if let Some(hub) = hubs_guard.find_by_name(hub_name) {
                hub_ids.push(hub.hub_id);
            }
        }
    }

    // Create the agent
    let agent = Agent {
        agent_id,
        name: body.name.clone(),
        ip_address: client_ip,
        port: client_port,
        endpoint: body.endpoint.clone(),
        status: AgentStatus::Active,
        description: body.description.clone(),
        capabilities: body.capabilities.clone(),
        hubs: hub_ids.clone(),
        avg_rating: rust_decimal::Decimal::ZERO,
        total_ratings: 0,
        created_at: now,
        updated_at: now,
        last_heartbeat: now,
    };

    // Store in memory
    {
        let mut agents_guard = arc.agents.lock();
        agents_guard.agents.insert(agent_id, agent);
    }

    // Store passkey mapping
    {
        let mut agent_passkeys_guard = arc.agent_passkeys.lock();
        agent_passkeys_guard.insert(passkey.to_vec(), agent_id);
    }

    // Persist to database
    let ip_network = sqlx::types::ipnetwork::IpNetwork::new(client_ip, 32).expect("Invalid IP");
    let capabilities_json = body.capabilities.clone();

    let _ = sqlx::query!(
        r#"
        INSERT INTO agents (agent_id, name, passkey, ip_address, port, endpoint, status, description, capabilities, created_at, updated_at, last_heartbeat)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
        ON CONFLICT (agent_id) DO UPDATE SET
            name = EXCLUDED.name,
            ip_address = EXCLUDED.ip_address,
            port = EXCLUDED.port,
            endpoint = EXCLUDED.endpoint,
            status = EXCLUDED.status,
            description = EXCLUDED.description,
            capabilities = EXCLUDED.capabilities,
            updated_at = EXCLUDED.updated_at
        "#,
        agent_id.0,
        body.name.clone(),
        &passkey[..],
        &ip_network,
        client_port as i32,
        body.endpoint.clone(),
        "active",
        body.description.clone(),
        &capabilities_json,
        now,
        now,
        now,
    )
    .execute(&arc.pool)
    .await;

    // Clear existing hub memberships and re-insert
    let _ = sqlx::query!(
        r#"
        DELETE FROM agent_hubs WHERE agent_id = $1
        "#,
        agent_id.0
    )
    .execute(&arc.pool)
    .await;

    // Insert into agent_hubs junction table
    for hub_id in &hub_ids {
        let _ = sqlx::query!(
            r#"
            INSERT INTO agent_hubs (agent_id, hub_id)
            VALUES ($1, $2)
            "#,
            agent_id.0,
            hub_id
        )
        .execute(&arc.pool)
        .await;
    }

    // Find other agents with similar capabilities for discovery
    let discovered_agents = {
        let agents_guard = arc.agents.lock();
        agents_guard
            .iter()
            .filter(|(id, a)| {
                *id != &agent_id
                    && a.status == AgentStatus::Active
                    && !a.capabilities.is_empty()
            })
            .take(10)
            .map(|(_, a)| DiscoveredAgent {
                agent_id: a.agent_id.to_string(),
                name: a.name.clone(),
                description: a.description.clone(),
                capabilities: a.capabilities.clone(),
                endpoint: a.endpoint.clone(),
                status: a.status.to_string(),
            })
            .collect()
    };

    // TODO: Persist to database asynchronously
    // For now, agent is only in memory

    HttpResponse::Ok().json(AgentRegisterResponse {
        status: "registered".to_string(),
        agent_id: agent_id.to_string(),
        agent_passkey: passkey_hex,
        interval: 60, // Heartbeat interval in seconds
        discovered_agents,
    })
}
