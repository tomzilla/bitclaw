use actix_web::{web::Data, web::Json, web::Path, HttpResponse};
use serde::{Deserialize, Serialize};
use sqlx::types::Uuid;
use utoipa::ToSchema;

use crate::Tracker;
use bitclaw_shared::tracker::models::agent::AgentId;

/// Hub information response
#[derive(Debug, Serialize, ToSchema)]
pub struct HubResponse {
    pub hub_id: String,
    pub name: String,
    pub description: Option<String>,
    pub max_agents: Option<i32>,
    pub is_public: bool,
    pub agent_count: usize,
}

/// List hubs response
#[derive(Debug, Serialize, ToSchema)]
pub struct ListHubsResponse {
    pub hubs: Vec<HubResponse>,
}

#[utoipa::path(
    get,
    operation_id = "list_hubs",
    tag = "Hubs",
    path = "/api/v1/hubs",
    responses(
        (status = 200, description = "List of hubs", body = ListHubsResponse),
    )
)]
pub async fn list_hubs(arc: Data<Tracker>) -> HttpResponse {
    let hubs_guard = arc.hubs.read();
    let agents_guard = arc.agents.lock();

    let hubs: Vec<HubResponse> = hubs_guard
        .iter()
        .map(|(_, hub)| {
            let agent_count = agents_guard
                .iter()
                .filter(|(_, agent)| agent.hubs.contains(&hub.hub_id))
                .count();

            HubResponse {
                hub_id: hub.hub_id.to_string(),
                name: hub.name.clone(),
                description: hub.description.clone(),
                max_agents: hub.max_agents,
                is_public: hub.is_public,
                agent_count,
            }
        })
        .collect();

    HttpResponse::Ok().json(ListHubsResponse { hubs })
}

/// Search request for finding agents in a hub
#[derive(Debug, Deserialize, ToSchema)]
pub struct HubSearchRequest {
    /// Keyword to search in agent descriptions
    pub q: String,
    /// Optional hub ID to search within (if not provided, searches all hubs)
    pub hub_id: Option<Uuid>,
    /// Maximum results to return (default: 50)
    pub limit: Option<usize>,
}

/// Agent search result for peer-to-peer discovery
#[derive(Debug, Serialize, ToSchema)]
pub struct DiscoverableAgent {
    pub agent_id: String,
    pub name: String,
    pub description: String,
    pub capabilities: Vec<String>,
    pub ip_address: String,
    pub port: u16,
    pub endpoint: Option<String>,
    pub avg_rating: String,
    pub total_ratings: u32,
}

/// Search response
#[derive(Debug, Serialize, ToSchema)]
pub struct HubSearchResponse {
    pub query: String,
    pub hub_id: Option<String>,
    pub agents: Vec<DiscoverableAgent>,
    pub total: usize,
}

#[utoipa::path(
    post,
    operation_id = "search_agents_in_hub",
    tag = "Hubs",
    path = "/api/v1/hubs/search",
    request_body = HubSearchRequest,
    responses(
        (status = 200, description = "Search results for peer-to-peer discovery", body = HubSearchResponse),
    )
)]
pub async fn search_agents(
    arc: Data<Tracker>,
    body: Json<HubSearchRequest>,
) -> HttpResponse {
    let agents_guard = arc.agents.lock();

    let hub_uuid = body.hub_id;
    let results = agents_guard.search_by_keyword(&body.q, hub_uuid.as_ref());

    let limit = body.limit.unwrap_or(50);
    let agents: Vec<DiscoverableAgent> = results
        .into_iter()
        .take(limit)
        .map(|agent| DiscoverableAgent {
            agent_id: agent.agent_id.to_string(),
            name: agent.name.clone(),
            description: agent.description.clone(),
            capabilities: agent.capabilities.clone(),
            ip_address: agent.ip_address.to_string(),
            port: agent.port,
            endpoint: agent.endpoint.clone(),
            avg_rating: agent.avg_rating.to_string(),
            total_ratings: agent.total_ratings,
        })
        .collect();

    let total = agents.len();

    HttpResponse::Ok().json(HubSearchResponse {
        query: body.q.clone(),
        hub_id: body.hub_id.map(|id| id.to_string()),
        agents,
        total,
    })
}

/// Get agents in a specific hub
#[derive(Debug, Serialize, ToSchema)]
pub struct HubAgentsResponse {
    pub hub: HubResponse,
    pub agents: Vec<DiscoverableAgent>,
}

#[utoipa::path(
    get,
    operation_id = "get_hub_agents",
    tag = "Hubs",
    path = "/api/v1/hubs/{hub_id}/agents",
    params(
        ("hub_id" = Uuid, Path, description = "Hub ID"),
    ),
    responses(
        (status = 200, description = "List of agents in hub", body = HubAgentsResponse),
        (status = 404, description = "Hub not found"),
    )
)]
pub async fn get_hub_agents(
    arc: Data<Tracker>,
    path: Path<Uuid>,
) -> HttpResponse {
    let hub_id = path.into_inner();

    let hubs_guard = arc.hubs.read();
    let hub = match hubs_guard.get(&hub_id) {
        Some(h) => h.clone(),
        None => {
            return HttpResponse::NotFound().json(serde_json::json!({
                "error": "Hub not found"
            }));
        }
    };

    let agents_guard = arc.agents.lock();

    let agents: Vec<DiscoverableAgent> = agents_guard
        .iter()
        .filter(|(_, agent)| {
            agent.hubs.contains(&hub_id) && agent.status.to_string() == "active"
        })
        .map(|(_, agent)| DiscoverableAgent {
            agent_id: agent.agent_id.to_string(),
            name: agent.name.clone(),
            description: agent.description.clone(),
            capabilities: agent.capabilities.clone(),
            ip_address: agent.ip_address.to_string(),
            port: agent.port,
            endpoint: agent.endpoint.clone(),
            avg_rating: agent.avg_rating.to_string(),
            total_ratings: agent.total_ratings,
        })
        .collect();

    let hub_response = HubResponse {
        hub_id: hub.hub_id.to_string(),
        name: hub.name,
        description: hub.description,
        max_agents: hub.max_agents,
        is_public: hub.is_public,
        agent_count: agents.len(),
    };

    HttpResponse::Ok().json(HubAgentsResponse {
        hub: hub_response,
        agents,
    })
}

/// Connect to hub request
#[derive(Debug, Deserialize, ToSchema)]
pub struct ConnectHubRequest {
    /// Client/Agent ID connecting to the hub
    pub client_id: String,
    /// Optional client name
    pub client_name: Option<String>,
    /// Optional address for peer-to-peer connections
    pub address: Option<String>,
}

/// Connect to hub response
#[derive(Debug, Serialize, ToSchema)]
pub struct ConnectHubResponse {
    pub status: String,
    pub hub_id: String,
    pub hub_name: String,
    pub client_id: String,
    pub message: String,
}

#[utoipa::path(
    post,
    operation_id = "connect_to_hub",
    tag = "Hubs",
    path = "/api/v1/hubs/{hub_id}/connect",
    params(
        ("hub_id" = Uuid, Path, description = "Hub ID"),
    ),
    request_body = ConnectHubRequest,
    responses(
        (status = 200, description = "Successfully connected to hub", body = ConnectHubResponse),
        (status = 404, description = "Hub not found"),
        (status = 400, description = "Invalid request"),
    )
)]
pub async fn connect_hub(
    arc: Data<Tracker>,
    path: Path<Uuid>,
    body: Json<ConnectHubRequest>,
) -> HttpResponse {
    let hub_id = path.into_inner();

    // Verify hub exists
    let hubs_guard = arc.hubs.read();
    let hub = match hubs_guard.get(&hub_id) {
        Some(h) => h,
        None => {
            return HttpResponse::NotFound().json(serde_json::json!({
                "error": "Hub not found"
            }));
        }
    };

    // Parse client_id
    let client_uuid = match Uuid::parse_str(&body.client_id) {
        Ok(id) => id,
        Err(_) => {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "error": "Invalid client_id format"
            }));
        }
    };

    // Find the agent and add hub to its list
    let mut agents_guard = arc.agents.lock();

    // Check if agent exists - use explicit AgentId type
    let agent_id = AgentId::from(client_uuid);
    let agent_exists = agents_guard.agents.contains_key(&agent_id);

    if !agent_exists {
        // Agent not registered - return error suggesting registration first
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": "Agent not registered. Please register first via /api/v1/agents"
        }));
    }

    // Add hub to agent's hub list if not already present
    if let Some(agent) = agents_guard.agents.get_mut(&agent_id) {
        if !agent.hubs.contains(&hub_id) {
            agent.hubs.push(hub_id);
            log::info!("Agent {} ({}) connected to hub '{}'", agent.name, agent.agent_id, hub.name);
        }
    }

    // Clone hub name before dropping guards
    let hub_name = hub.name.clone();
    drop(agents_guard);
    drop(hubs_guard);

    // Persist hub connection to database
    let _ = sqlx::query!(
        r#"
        INSERT INTO agent_hubs (agent_id, hub_id)
        VALUES ($1, $2)
        ON CONFLICT (agent_id, hub_id) DO NOTHING
        "#,
        client_uuid,
        hub_id
    )
    .execute(&arc.pool)
    .await;

    let message = format!("Successfully connected to hub '{}'", hub_name);
    let response = ConnectHubResponse {
        status: "connected".to_string(),
        hub_id: hub_id.to_string(),
        hub_name: hub_name.clone(),
        client_id: body.client_id.clone(),
        message,
    };

    HttpResponse::Ok().json(response)
}

/// Disconnect from hub request
#[derive(Debug, Deserialize, ToSchema)]
pub struct DisconnectHubRequest {
    /// Client/Agent ID disconnecting from the hub
    pub client_id: String,
}

/// Disconnect from hub response
#[derive(Debug, Serialize, ToSchema)]
pub struct DisconnectHubResponse {
    pub status: String,
    pub hub_id: String,
    pub hub_name: String,
    pub client_id: String,
    pub message: String,
}

#[utoipa::path(
    post,
    operation_id = "disconnect_from_hub",
    tag = "Hubs",
    path = "/api/v1/hubs/{hub_id}/disconnect",
    params(
        ("hub_id" = Uuid, Path, description = "Hub ID"),
    ),
    request_body = DisconnectHubRequest,
    responses(
        (status = 200, description = "Successfully disconnected from hub", body = DisconnectHubResponse),
        (status = 404, description = "Hub not found"),
    )
)]
pub async fn disconnect_hub(
    arc: Data<Tracker>,
    path: Path<Uuid>,
    body: Json<DisconnectHubRequest>,
) -> HttpResponse {
    let hub_id = path.into_inner();

    // Verify hub exists
    let hubs_guard = arc.hubs.read();
    let hub = match hubs_guard.get(&hub_id) {
        Some(h) => h,
        None => {
            return HttpResponse::NotFound().json(serde_json::json!({
                "error": "Hub not found"
            }));
        }
    };

    // Parse client_id
    let client_uuid = match Uuid::parse_str(&body.client_id) {
        Ok(id) => id,
        Err(_) => {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "error": "Invalid client_id format"
            }));
        }
    };

    // Remove hub from agent's hub list
    let mut agents_guard = arc.agents.lock();

    let agent_id = AgentId::from(client_uuid);
    if let Some(agent) = agents_guard.agents.get_mut(&agent_id) {
        agent.hubs.retain(|h| *h != hub_id);
        log::info!("Agent {} ({}) disconnected from hub '{}'", agent.name, agent.agent_id, hub.name);
    }

    // Clone hub name before dropping guards
    let hub_name = hub.name.clone();
    drop(agents_guard);
    drop(hubs_guard);

    // Remove hub connection from database
    let _ = sqlx::query!(
        r#"
        DELETE FROM agent_hubs WHERE agent_id = $1 AND hub_id = $2
        "#,
        client_uuid,
        hub_id
    )
    .execute(&arc.pool)
    .await;

    let message = format!("Successfully disconnected from hub '{}'", hub_name);
    let response = DisconnectHubResponse {
        status: "disconnected".to_string(),
        hub_id: hub_id.to_string(),
        hub_name: hub_name.clone(),
        client_id: body.client_id.clone(),
        message,
    };

    HttpResponse::Ok().json(response)
}
