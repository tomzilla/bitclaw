use actix_web::{web, web::Data, HttpResponse};
use chrono::Utc;
use num_traits::cast::ToPrimitive;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::Tracker;

/// Request body for rating an agent
#[derive(Debug, Deserialize, ToSchema)]
pub struct RateAgentRequest {
    /// The agent submitting the rating
    pub rater_agent_id: Uuid,
    /// The agent being rated
    pub rated_agent_id: Uuid,
    /// Star rating from 1 to 5
    pub stars: u8,
    /// Optional comment
    pub comment: Option<String>,
}

/// Response for rating an agent
#[derive(Debug, Serialize, ToSchema)]
pub struct RateAgentResponse {
    pub status: String,
    pub rated_agent_id: String,
    pub new_avg_rating: f64,
    pub new_total_ratings: i64,
}

/// Agent rating info
#[derive(Debug, Serialize, ToSchema)]
pub struct AgentRatingInfo {
    pub rater_agent_id: String,
    pub rater_name: String,
    pub stars: i32,
    pub comment: Option<String>,
    pub created_at: String,
}

/// Response for getting agent ratings
#[derive(Debug, Serialize, ToSchema)]
pub struct GetRatingsResponse {
    pub agent_id: String,
    pub agent_name: String,
    pub avg_rating: f64,
    pub total_ratings: i32,
    pub ratings: Vec<AgentRatingInfo>,
}

#[utoipa::path(
    post,
    operation_id = "rate_agent",
    tag = "Agents",
    path = "/api/v1/agents/rate",
    request_body = RateAgentRequest,
    responses(
        (status = 200, description = "Agent rated successfully", body = RateAgentResponse),
        (status = 400, description = "Invalid rating"),
        (status = 404, description = "Agent not found"),
    )
)]
pub async fn rate(
    arc: Data<Tracker>,
    body: web::Json<RateAgentRequest>,
) -> HttpResponse {
    // Validate star rating
    if body.stars < 1 || body.stars > 5 {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": "Stars must be between 1 and 5"
        }));
    }

    // Validate that rater and rated are not the same
    if body.rater_agent_id == body.rated_agent_id {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": "An agent cannot rate itself"
        }));
    }

    let now = Utc::now();

    // Insert or update the rating
    let result = sqlx::query!(
        r#"
        INSERT INTO agent_ratings (rater_agent_id, rated_agent_id, stars, comment, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6)
        ON CONFLICT (rater_agent_id, rated_agent_id)
        DO UPDATE SET
            stars = EXCLUDED.stars,
            comment = EXCLUDED.comment,
            updated_at = EXCLUDED.updated_at
        RETURNING rated_agent_id
        "#,
        body.rater_agent_id,
        body.rated_agent_id,
        body.stars as i32,
        body.comment.as_deref(),
        now,
        now
    )
    .fetch_optional(&arc.pool)
    .await;

    match result {
        Ok(Some(row)) => {
            // Fetch updated rating stats
            let stats = sqlx::query!(
                r#"
                SELECT avg_rating, total_ratings
                FROM agents
                WHERE agent_id = $1
                "#,
                body.rated_agent_id
            )
            .fetch_optional(&arc.pool)
            .await;

            match stats {
                Ok(Some(stats)) => HttpResponse::Ok().json(RateAgentResponse {
                    status: "rated".to_string(),
                    rated_agent_id: body.rated_agent_id.to_string(),
                    new_avg_rating: stats.avg_rating.unwrap_or_default().to_f64().unwrap_or(0.0),
                    new_total_ratings: stats.total_ratings.unwrap_or(0) as i64,
                }),
                _ => HttpResponse::NotFound().json(serde_json::json!({
                    "error": "Rated agent not found"
                })),
            }
        }
        Ok(None) => {
            HttpResponse::NotFound().json(serde_json::json!({
                "error": "Failed to create rating"
            }))
        }
        Err(_) => {
            // Check if it's because agents don't exist
            HttpResponse::NotFound().json(serde_json::json!({
                "error": "One or more agents not found"
            }))
        }
    }
}

#[utoipa::path(
    get,
    operation_id = "get_agent_ratings",
    tag = "Agents",
    path = "/api/v1/agents/{agent_id}/ratings",
    params(
        ("agent_id" = Uuid, Path, description = "Agent ID"),
    ),
    responses(
        (status = 200, description = "Agent ratings", body = GetRatingsResponse),
        (status = 404, description = "Agent not found"),
    )
)]
pub async fn get_ratings(
    arc: Data<Tracker>,
    path: web::Path<Uuid>,
) -> HttpResponse {
    let agent_id = path.into_inner();

    // Get agent info
    let agent_info = sqlx::query!(
        r#"
        SELECT agent_id, name, avg_rating, total_ratings
        FROM agents
        WHERE agent_id = $1
        "#,
        agent_id
    )
    .fetch_optional(&arc.pool)
    .await;

    match agent_info {
        Ok(Some(agent)) => {
            // Get ratings
            let ratings = sqlx::query!(
                r#"
                SELECT
                    r.rater_agent_id,
                    a.name as "rater_name!",
                    r.stars,
                    r.comment,
                    r.created_at
                FROM agent_ratings r
                JOIN agents a ON r.rater_agent_id = a.agent_id
                WHERE r.rated_agent_id = $1
                ORDER BY r.created_at DESC
                LIMIT 50
                "#,
                agent_id
            )
            .fetch_all(&arc.pool)
            .await
            .unwrap_or_default();

            let ratings_list: Vec<AgentRatingInfo> = ratings
                .into_iter()
                .map(|r| AgentRatingInfo {
                    rater_agent_id: r.rater_agent_id.to_string(),
                    rater_name: r.rater_name,
                    stars: r.stars,
                    comment: r.comment,
                    created_at: r.created_at.to_rfc3339(),
                })
                .collect();

            HttpResponse::Ok().json(GetRatingsResponse {
                agent_id: agent.agent_id.to_string(),
                agent_name: agent.name,
                avg_rating: agent.avg_rating.unwrap_or_default().to_f64().unwrap_or(0.0),
                total_ratings: agent.total_ratings.unwrap_or(0) as i32,
                ratings: ratings_list,
            })
        }
        _ => HttpResponse::NotFound().json(serde_json::json!({
            "error": "Agent not found"
        })),
    }
}
