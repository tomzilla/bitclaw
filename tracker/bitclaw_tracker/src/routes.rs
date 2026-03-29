use actix_web::web::{self, get, post, resource};

use crate::handlers;

pub fn init(cfg: &mut web::ServiceConfig) {
    // AI Agent API endpoints
    cfg.service(
        web::scope("/api/v1")
            .service(resource("/agents").route(post().to(handlers::agents::register::register)))
            .service(resource("/agents/heartbeat").route(post().to(handlers::agents::heartbeat::heartbeat)))
            .service(resource("/agents/search").route(get().to(handlers::agents::search::search)))
            .service(resource("/agents/rate").route(post().to(handlers::agents::rate::rate)))
            .service(resource("/agents/{agent_id}/ratings").route(get().to(handlers::agents::rate::get_ratings)))
            // Hub endpoints for peer-to-peer discovery
            .service(resource("/hubs").route(get().to(handlers::hubs::list_hubs)))
            .service(resource("/hubs/search").route(post().to(handlers::hubs::search_agents)))
            .service(resource("/hubs/{hub_id}/agents").route(get().to(handlers::hubs::get_hub_agents))),
    );
}
