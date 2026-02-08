use actix_web::{get, post, web, HttpResponse};
use serde::{Deserialize, Serialize};

use crate::{error::Result, monitor::HealthMonitor, session::SessionManager};

#[derive(Debug, Deserialize)]
pub struct HeartbeatRequest {
    pub world_id: String,
    pub current_players: u32,
    pub timestamp: u64,
}

#[derive(Debug, Serialize)]
pub struct HeartbeatResponse {
    pub success: bool,
    pub next_heartbeat_in: u64,
}

#[post("/heartbeat")]
pub async fn heartbeat(
    req: web::Json<HeartbeatRequest>,
    health_monitor: web::Data<HealthMonitor>,
) -> Result<HttpResponse> {
    health_monitor.record_heartbeat(req.world_id.clone(), req.current_players);

    log::debug!(
        "Heartbeat received from world {} ({} players)",
        req.world_id,
        req.current_players
    );

    let response = HeartbeatResponse {
        success: true,
        next_heartbeat_in: 15, // Game servers should send heartbeat every 15 seconds
    };

    Ok(HttpResponse::Ok().json(response))
}

#[derive(Debug, Serialize)]
pub struct HealthCheckResponse {
    pub status: String,
    pub active_sessions: usize,
    pub online_worlds: usize,
}

#[get("/health")]
pub async fn health_check(
    health_monitor: web::Data<HealthMonitor>,
    session_manager: Option<web::Data<SessionManager>>,
) -> Result<HttpResponse> {
    let online_worlds = health_monitor.online_world_count();
    let active_sessions = session_manager
        .map(|manager| manager.active_session_count())
        .unwrap_or(0);

    let response = HealthCheckResponse {
        status: "healthy".to_string(),
        active_sessions,
        online_worlds,
    };

    Ok(HttpResponse::Ok().json(response))
}
