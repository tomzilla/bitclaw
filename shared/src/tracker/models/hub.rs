use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::ops::{Deref, DerefMut};

/// Hub that agents can join for group communication
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Hub {
    pub hub_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub max_agents: Option<i32>,
    pub is_public: bool,
}

/// Map of hubs indexed by hub_id
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HubMap {
    pub hubs: indexmap::IndexMap<Uuid, Hub>,
}

impl HubMap {
    pub fn new() -> Self {
        Self {
            hubs: indexmap::IndexMap::new(),
        }
    }

    /// Find a hub by name
    pub fn find_by_name(&self, name: &str) -> Option<&Hub> {
        self.hubs.values().find(|hub| hub.name.eq_ignore_ascii_case(name))
    }

    /// Load hubs from database
    pub async fn from_database(db: &sqlx::PgPool) -> Self {
        let rows = sqlx::query!(
            r#"
            SELECT hub_id, name, description, max_agents, is_public
            FROM hubs
            "#
        )
        .fetch_all(db)
        .await
        .expect("could not get hubs");

        let mut map = HubMap::new();
        for r in rows {
            let hub = Hub {
                hub_id: r.hub_id,
                name: r.name,
                description: r.description,
                max_agents: r.max_agents,
                is_public: r.is_public,
            };
            map.hubs.insert(hub.hub_id, hub);
        }

        map
    }
}

impl Default for HubMap {
    fn default() -> Self {
        Self::new()
    }
}

impl Deref for HubMap {
    type Target = indexmap::IndexMap<Uuid, Hub>;

    fn deref(&self) -> &Self::Target {
        &self.hubs
    }
}

impl DerefMut for HubMap {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.hubs
    }
}
