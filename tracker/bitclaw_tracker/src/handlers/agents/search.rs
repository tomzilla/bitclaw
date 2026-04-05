use actix_web::{web::Data, web::Query, HttpResponse};
use num_traits::cast::ToPrimitive;
use serde::{Deserialize, Serialize};
use sqlx::types::Uuid;
use utoipa::ToSchema;

use crate::Tracker;

/// Query parameters for agent search
#[derive(Debug, Deserialize, ToSchema)]
pub struct SearchAgentsQuery {
    /// Search by capability tag
    pub capability: Option<String>,
    /// Search by hub ID
    pub hub: Option<Uuid>,
    /// Search by description text (keyword search)
    pub q: Option<String>,
    /// Filter by status
    pub status: Option<String>,
    /// Maximum results to return (default: 50)
    pub limit: Option<usize>,
    /// Sort by rating (default: false)
    pub sort_by_rating: Option<bool>,
}

/// Response for agent search
#[derive(Debug, Serialize, ToSchema)]
pub struct SearchAgentsResponse {
    pub agents: Vec<AgentResult>,
    pub total: usize,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AgentResult {
    pub agent_id: String,
    pub name: String,
    pub description: String,
    pub capabilities: Vec<String>,
    pub hubs: Vec<Uuid>,
    pub endpoint: Option<String>,
    pub ip_address: String,
    pub port: u16,
    pub status: String,
    pub avg_rating: String,
    pub total_ratings: i32,
    pub last_heartbeat: String,
}

#[utoipa::path(
    get,
    operation_id = "search_agents",
    tag = "Agents",
    path = "/api/v1/agents/search",
    params(
        ("capability" = Option<String>, Query, description = "Search by capability tag"),
        ("hub" = Option<Uuid>, Query, description = "Search by hub ID"),
        ("q" = Option<String>, Query, description = "Search by description text (keyword search)"),
        ("status" = Option<String>, Query, description = "Filter by status"),
        ("limit" = Option<usize>, Query, description = "Maximum results"),
        ("sort_by_rating" = Option<bool>, Query, description = "Sort by rating"),
    ),
    responses(
        (status = 200, description = "Search results", body = SearchAgentsResponse),
    )
)]
pub async fn search(
    arc: Data<Tracker>,
    query: Query<SearchAgentsQuery>,
) -> HttpResponse {
    search_from_memory(&arc, &query).await
}

async fn search_from_memory(
    arc: &Data<Tracker>,
    query: &SearchAgentsQuery,
) -> HttpResponse {
    let agents_guard = arc.agents.lock();

    let mut results: Vec<(AgentResult, f64)> = agents_guard
        .iter()
        .filter_map(|(_, agent)| {
            // Filter by status if provided
            if let Some(ref status) = query.status {
                let agent_status = agent.status.to_string();
                if !agent_status.eq_ignore_ascii_case(status) {
                    return None;
                }
            }

            // Filter by capability if provided
            if let Some(ref capability) = query.capability {
                if !agent.capabilities.iter().any(|c| c.eq_ignore_ascii_case(capability)) {
                    return None;
                }
            }

            // Filter by hub if provided
            if let Some(ref hub_id) = query.hub {
                if !agent.hubs.contains(hub_id) {
                    return None;
                }
            }

            // Keyword search in description and capabilities
            if let Some(ref q) = query.q {
                let q_lower = q.to_lowercase();
                let description_match = agent.description.to_lowercase().contains(&q_lower);
                let capability_match = agent.capabilities.iter().any(|c| c.to_lowercase().contains(&q_lower));
                let name_match = agent.name.to_lowercase().contains(&q_lower);

                if !description_match && !capability_match && !name_match {
                    return None;
                }

                // Calculate relevance score
                let mut score = 0.0;
                if name_match { score += 3.0; }
                if description_match { score += 2.0; }
                if capability_match { score += 1.0; }

                // Boost by rating
                score += agent.avg_rating.to_f64().unwrap_or(0.0);

                return Some((
                    AgentResult {
                        agent_id: agent.agent_id.to_string(),
                        name: agent.name.clone(),
                        description: agent.description.clone(),
                        capabilities: agent.capabilities.clone(),
                        hubs: agent.hubs.clone(),
                        endpoint: agent.endpoint.clone(),
                        ip_address: agent.ip_address.to_string(),
                        port: agent.port,
                        status: agent.status.to_string(),
                        avg_rating: agent.avg_rating.to_string(),
                        total_ratings: agent.total_ratings as i32,
                        last_heartbeat: agent.last_heartbeat.to_rfc3339(),
                    },
                    score,
                ));
            }

            Some((
                AgentResult {
                    agent_id: agent.agent_id.to_string(),
                    name: agent.name.clone(),
                    description: agent.description.clone(),
                    capabilities: agent.capabilities.clone(),
                    hubs: agent.hubs.clone(),
                    endpoint: agent.endpoint.clone(),
                    ip_address: agent.ip_address.to_string(),
                    port: agent.port,
                    status: agent.status.to_string(),
                    avg_rating: agent.avg_rating.to_string(),
                    total_ratings: agent.total_ratings as i32,
                    last_heartbeat: agent.last_heartbeat.to_rfc3339(),
                },
                agent.avg_rating.to_f64().unwrap_or(0.0),
            ))
        })
        .collect();

    // Sort by relevance/rating if sort_by_rating is true or if there's a search query
    if query.sort_by_rating.unwrap_or(false) || query.q.is_some() {
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    }

    // Take only the requested limit
    let limit = query.limit.unwrap_or(50);
    results.truncate(limit);

    let total = results.len();
    let agents: Vec<AgentResult> = results.into_iter().map(|(r, _)| r).collect();

    HttpResponse::Ok().json(SearchAgentsResponse { agents, total })
}
