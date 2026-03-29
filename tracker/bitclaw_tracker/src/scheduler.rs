use actix_web::web::Data;
use chrono::{Duration, Utc};

use crate::Tracker;

pub async fn handle(arc: &Data<Tracker>) {
    let mut interval = tokio::time::interval(std::time::Duration::from_millis(1));
    let mut counter = 0_u64;

    loop {
        interval.tick().await;
        counter += 1;

        // Clean up inactive agents every 60 seconds
        if counter.is_multiple_of(60_000) {
            reap_agents(arc).await;
        }
    }
}

/// Remove agents that have not sent a heartbeat for some time
pub async fn reap_agents(arc: &Data<Tracker>) {
    let agent_ttl_seconds = arc.env.agent_heartbeat_ttl_seconds as i64;
    let ttl = Duration::seconds(agent_ttl_seconds);
    let cutoff = Utc::now().checked_sub_signed(ttl).unwrap();

    let mut agents_to_remove = Vec::new();

    // Find agents that have exceeded the heartbeat TTL
    {
        let mut agents_guard = arc.agents.lock();
        agents_to_remove = agents_guard
            .agents
            .iter()
            .filter(|(_, agent)| agent.last_heartbeat < cutoff)
            .map(|(id, _)| *id)
            .collect::<Vec<_>>();

        // Remove inactive agents from memory
        for agent_id in &agents_to_remove {
            agents_guard.agents.swap_remove(agent_id);
        }
    }

    if !agents_to_remove.is_empty() {
        log::info!(
            "Removed {} inactive agents (no heartbeat for {} seconds)",
            agents_to_remove.len(),
            agent_ttl_seconds
        );
    }
}
