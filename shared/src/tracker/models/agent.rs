use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::{
    fmt::{Debug, Display},
    ops::{Deref, DerefMut},
    str::FromStr,
};

/// Unique identifier for an AI agent
#[derive(Clone, Copy, Eq, Hash, PartialEq, PartialOrd, Ord)]
pub struct AgentId(pub Uuid);

impl AgentId {
    pub fn new() -> Self {
        AgentId(Uuid::new_v4())
    }
}

impl Default for AgentId {
    fn default() -> Self {
        Self::new()
    }
}

impl Deref for AgentId {
    type Target = Uuid;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<Uuid> for AgentId {
    fn from(uuid: Uuid) -> Self {
        AgentId(uuid)
    }
}

impl FromStr for AgentId {
    type Err = uuid::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(AgentId(Uuid::from_str(s)?))
    }
}

impl Display for AgentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0.to_string())
    }
}

impl Debug for AgentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.to_string())
    }
}

impl Serialize for AgentId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for AgentId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        AgentId::from_str(&s).map_err(serde::de::Error::custom)
    }
}

/// Agent status indicating current state
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AgentStatus {
    Active,
    Busy,
    Inactive,
    Offline,
}

impl Default for AgentStatus {
    fn default() -> Self {
        AgentStatus::Active
    }
}

impl Display for AgentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentStatus::Active => write!(f, "active"),
            AgentStatus::Busy => write!(f, "busy"),
            AgentStatus::Inactive => write!(f, "inactive"),
            AgentStatus::Offline => write!(f, "offline"),
        }
    }
}

/// Agent record for in-memory tracking
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Agent {
    pub agent_id: AgentId,
    pub name: String,
    pub ip_address: std::net::IpAddr,
    pub port: u16,
    pub endpoint: Option<String>,
    pub status: AgentStatus,
    pub description: String,
    pub capabilities: Vec<String>,
    pub hubs: Vec<Uuid>,
    pub avg_rating: rust_decimal::Decimal,
    pub total_ratings: u32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_heartbeat: DateTime<Utc>,
}

impl Agent {
    /// Determines if the agent is currently available for communication
    #[inline(always)]
    pub fn is_available(&self) -> bool {
        self.status == AgentStatus::Active
    }

    /// Determines if the agent should be considered inactive based on heartbeat
    pub fn is_stale(&self, threshold_seconds: i64) -> bool {
        Utc::now()
            .signed_duration_since(self.last_heartbeat)
            .num_seconds()
            > threshold_seconds
    }
}

/// Index for agent lookup
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct AgentIndex {
    pub agent_id: AgentId,
}

/// Map of agents indexed by agent_id
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentMap {
    pub agents: indexmap::IndexMap<AgentId, Agent>,
}

impl AgentMap {
    pub fn new() -> Self {
        Self {
            agents: indexmap::IndexMap::new(),
        }
    }

    /// Find agents by capability tag
    pub fn find_by_capability(&self, capability: &str) -> Vec<&Agent> {
        self.agents
            .values()
            .filter(|agent| {
                agent
                    .capabilities
                    .iter()
                    .any(|c| c.eq_ignore_ascii_case(capability))
            })
            .collect()
    }

    /// Find agents by hub
    pub fn find_by_hub(&self, hub_id: &Uuid) -> Vec<&Agent> {
        self.agents
            .values()
            .filter(|agent| agent.hubs.contains(hub_id))
            .collect()
    }

    /// Search agents by keyword - matches against description, name, and capabilities
    /// Used for peer-to-peer discovery within a hub
    pub fn search_by_keyword(&self, query: &str, hub_id: Option<&Uuid>) -> Vec<&Agent> {
        let query_lower = query.to_lowercase();
        self.agents
            .values()
            .filter(|agent| {
                // Filter by hub if specified
                if let Some(hid) = hub_id {
                    if !agent.hubs.contains(hid) {
                        return false;
                    }
                }

                // Must be active to be discoverable
                if !agent.is_available() {
                    return false;
                }

                // Search in description, name, and capabilities
                agent.description.to_lowercase().contains(&query_lower)
                    || agent.name.to_lowercase().contains(&query_lower)
                    || agent.capabilities.iter().any(|c| c.to_lowercase().contains(&query_lower))
            })
            .collect()
    }

    /// Load agents from database
    pub async fn from_database(db: &sqlx::PgPool) -> Self {
        let rows = sqlx::query_as!(
            DbImportAgent,
            r#"
            SELECT
                agent_id,
                name,
                passkey,
                ip_address,
                port,
                endpoint,
                status,
                description,
                capabilities,
                COALESCE(avg_rating, 0) as "avg_rating!: rust_decimal::Decimal",
                total_ratings,
                created_at,
                updated_at,
                last_heartbeat
            FROM agents
            "#
        )
        .fetch_all(db)
        .await
        .expect("could not get agents");

        let mut map = AgentMap::new();
        for r in rows {
            let agent = Agent {
                agent_id: AgentId(r.agent_id),
                name: r.name,
                ip_address: match r.ip_address {
                    sqlx::types::ipnetwork::IpNetwork::V4(v4) => v4.ip().into(),
                    sqlx::types::ipnetwork::IpNetwork::V6(v6) => v6.ip().into(),
                },
                port: r.port as u16,
                endpoint: r.endpoint,
                status: match r.status.as_str() {
                    "active" => AgentStatus::Active,
                    "busy" => AgentStatus::Busy,
                    "inactive" => AgentStatus::Inactive,
                    _ => AgentStatus::Offline,
                },
                description: r.description,
                capabilities: r.capabilities,
                hubs: Vec::new(), // Will be populated separately
                avg_rating: r.avg_rating,
                total_ratings: r.total_ratings.unwrap_or(0) as u32,
                created_at: r.created_at,
                updated_at: r.updated_at,
                last_heartbeat: r.last_heartbeat,
            };
            map.agents.insert(agent.agent_id, agent);
        }

        // Load agent-hub memberships
        let agent_hubs = sqlx::query!(
            r#"
            SELECT agent_id, hub_id
            FROM agent_hubs
            "#
        )
        .fetch_all(db)
        .await
        .expect("could not get agent_hubs");

        for row in agent_hubs {
            let agent_id = AgentId(row.agent_id);
            let hub_id = row.hub_id;
            if let Some(agent) = map.agents.get_mut(&agent_id) {
                agent.hubs.push(hub_id);
            }
        }

        map
    }
}

impl Default for AgentMap {
    fn default() -> Self {
        Self::new()
    }
}

impl Deref for AgentMap {
    type Target = indexmap::IndexMap<AgentId, Agent>;

    fn deref(&self) -> &Self::Target {
        &self.agents
    }
}

impl DerefMut for AgentMap {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.agents
    }
}

/// Database import structure for agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbImportAgent {
    pub agent_id: Uuid,
    pub name: String,
    pub passkey: Vec<u8>,
    pub ip_address: sqlx::types::ipnetwork::IpNetwork,
    pub port: i32,
    pub endpoint: Option<String>,
    pub status: String,
    pub description: String,
    pub capabilities: Vec<String>,
    pub avg_rating: rust_decimal::Decimal,
    pub total_ratings: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_heartbeat: DateTime<Utc>,
}
