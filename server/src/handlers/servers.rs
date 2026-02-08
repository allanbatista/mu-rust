use actix_web::{get, web, HttpResponse};
use serde::Serialize;

use crate::{config::ServerConfig, error::Result, monitor::HealthMonitor};

#[derive(Debug, Serialize)]
pub struct ServerListResponse {
    pub servers: Vec<ServerInfo>,
}

#[derive(Debug, Serialize)]
pub struct ServerInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub status: String,
    pub world_count: usize,
}

#[get("/servers")]
pub async fn list_servers(
    config: web::Data<ServerConfig>,
    health_monitor: web::Data<HealthMonitor>,
) -> Result<HttpResponse> {
    let servers: Vec<ServerInfo> = config
        .servers
        .iter()
        .map(|server| {
            // Count online worlds for this server
            let online_worlds = server
                .worlds
                .iter()
                .filter(|w| health_monitor.is_world_online(&w.id))
                .count();

            // Server is online if it has at least one online world
            let status = if online_worlds > 0 {
                "online"
            } else {
                "offline"
            };

            ServerInfo {
                id: server.id.clone(),
                name: server.name.clone(),
                description: server.description.clone(),
                status: status.to_string(),
                world_count: online_worlds,
            }
        })
        .collect();

    let response = ServerListResponse { servers };

    Ok(HttpResponse::Ok().json(response))
}

#[derive(Debug, Serialize)]
pub struct WorldListResponse {
    pub worlds: Vec<WorldInfo>,
}

#[derive(Debug, Serialize)]
pub struct WorldInfo {
    pub id: String,
    pub name: String,
    pub server_id: String,
    pub ip: String,
    pub port: u16,
    pub status: String,
    pub current_players: u32,
    pub max_players: u32,
}

#[get("/worlds")]
pub async fn list_worlds(
    config: web::Data<ServerConfig>,
    health_monitor: web::Data<HealthMonitor>,
) -> Result<HttpResponse> {
    let mut worlds = Vec::new();

    for server in &config.servers {
        for world in &server.worlds {
            let (is_online, current_players) = health_monitor
                .get_world_status(&world.id)
                .unwrap_or((false, 0));

            // Only include online worlds
            if is_online {
                worlds.push(WorldInfo {
                    id: world.id.clone(),
                    name: world.name.clone(),
                    server_id: server.id.clone(),
                    ip: world.ip.clone(),
                    port: world.port,
                    status: "online".to_string(),
                    current_players,
                    max_players: world.max_players,
                });
            }
        }
    }

    let response = WorldListResponse { worlds };

    Ok(HttpResponse::Ok().json(response))
}
