use arcadia_shared::tracker::models::{
    agent::AgentMap,
    hub::HubMap,
    env::ArcadiaSettingsForTracker,
};
use parking_lot::{Mutex, RwLock};
use sqlx::{postgres::PgPoolOptions, PgPool};

use crate::env::Env;
use std::{collections::HashMap, io::Write, ops::Deref, sync::OnceLock, time::Duration};

pub mod api_doc;
pub mod env;
pub mod handlers;
pub mod metrics;
pub mod middleware;
pub mod routes;
pub mod scheduler;

#[derive(Debug)]
pub struct Tracker {
    pub env: Env,
    pub pool: PgPool,
    pub settings: RwLock<ArcadiaSettingsForTracker>,
    pub metrics: OnceLock<metrics::Instruments>,

    // AI agent tracker fields
    pub agents: Mutex<AgentMap>,
    pub hubs: RwLock<HubMap>,
    pub agent_passkeys: Mutex<HashMap<Vec<u8>, arcadia_shared::tracker::models::agent::AgentId>>,
}

impl Deref for Tracker {
    type Target = Env;

    fn deref(&self) -> &Self::Target {
        &self.env
    }
}

impl Tracker {
    pub async fn new(env: Env) -> Self {
        println!("{:?}", env);

        print!("Connecting to database... ");
        std::io::stdout().flush().unwrap();
        let pool = connect_to_database().await;
        println!("[Finished]");

        log::info!("[Setup] Getting agents...");
        std::io::stdout().flush().unwrap();
        let agents = AgentMap::from_database(&pool).await;
        log::info!("[Setup] Got {:?} agents", agents.len());

        log::info!("[Setup] Getting hubs...");
        std::io::stdout().flush().unwrap();
        let hubs = HubMap::from_database(&pool).await;
        log::info!("[Setup] Got {:?} hubs", hubs.len());

        Self {
            env,
            pool,
            settings: RwLock::new(ArcadiaSettingsForTracker::default()),
            metrics: OnceLock::new(),
            agents: Mutex::new(agents),
            hubs: RwLock::new(hubs),
            agent_passkeys: Mutex::new(HashMap::new()),
        }
    }
}

async fn connect_to_database() -> sqlx::Pool<sqlx::Postgres> {
    PgPoolOptions::new()
        .min_connections(0)
        .max_connections(60)
        .max_lifetime(Duration::from_secs(30 * 60))
        .idle_timeout(Duration::from_secs(10 * 60))
        .acquire_timeout(Duration::from_secs(30))
        .connect(
            &std::env::var("DATABASE_URL").expect("DATABASE_URL not found in .env file. Aborting.")
        )
        .await
        .expect("Could not connect to the database using the DATABASE_URL value in .env file. Aborting.")
}
