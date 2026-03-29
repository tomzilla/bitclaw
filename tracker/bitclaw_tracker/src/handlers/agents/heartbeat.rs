use actix_web::{web, web::Data, HttpResponse};
use arcadia_shared::tracker::models::agent::AgentStatus;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::Tracker;

/// Request body for agent heartbeat
#[derive(Debug, Deserialize, ToSchema)]
pub struct HeartbeatRequest {
    pub agent_id: Uuid,
    pub status: Option<String>,
    pub endpoint: Option<String>,
}

/// Response for agent heartbeat
#[derive(Debug, Serialize, ToSchema)]
pub struct HeartbeatResponse {
    pub status: String,
    pub interval: u64,
    pub discovered_agents: Vec<DiscoveredAgent>,
}

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
    operation_id = "agent_heartbeat",
    tag = "Agents",
    path = "/api/v1/agents/heartbeat",
    request_body = HeartbeatRequest,
    responses(
        (status = 200, description = "Heartbeat recorded", body = HeartbeatResponse),
        (status = 404, description = "Agent not found"),
    )
)]
pub async fn heartbeat(
    arc: Data<Tracker>,
    body: web::Json<HeartbeatRequest>,
) -> HttpResponse {
    let agent_id = uuid::Uuid::from_u128(body.agent_id.as_u128());
    let agent_id = arcadia_shared::tracker::models::agent::AgentId(agent_id);

    let now = Utc::now();

    // Update agent's heartbeat timestamp
    {
        let mut agents_guard = arc.agents.lock();
        if let Some(agent) = agents_guard.agents.get_mut(&agent_id) {
            agent.last_heartbeat = now;

            // Update status if provided
            if let Some(ref new_status) = body.status {
                agent.status = match new_status.as_str() {
                    "active" => AgentStatus::Active,
                    "busy" => AgentStatus::Busy,
                    "inactive" => AgentStatus::Inactive,
                    _ => AgentStatus::Offline,
                };
            }

            // Update endpoint if provided
            if let Some(ref new_endpoint) = body.endpoint {
                agent.endpoint = Some(new_endpoint.clone());
            }

            agent.updated_at = now;
        } else {
            return HttpResponse::NotFound().json(serde_json::json!({
                "error": "Agent not found"
            }));
        }
    }

    // Find other active agents for discovery
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

    HttpResponse::Ok().json(HeartbeatResponse {
        status: "ok".to_string(),
        interval: 60,
        discovered_agents,
    })
}
