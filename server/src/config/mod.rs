use serde::Deserialize;
use std::fs;
use std::path::Path;

use crate::error::{ConnectServerError, Result};

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub servers: Vec<GameServer>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GameServer {
    pub id: String,
    pub name: String,
    pub description: String,
    pub worlds: Vec<WorldServer>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WorldServer {
    pub id: String,
    pub name: String,
    pub ip: String,
    pub port: u16,
    pub max_players: u32,
}

impl ServerConfig {
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(path).map_err(|e| {
            ConnectServerError::Config(format!("Failed to read config file: {}", e))
        })?;

        let config: ServerConfig = toml::from_str(&content).map_err(|e| {
            ConnectServerError::Config(format!("Failed to parse config file: {}", e))
        })?;

        Ok(config)
    }

    #[cfg(test)]
    pub fn get_server(&self, server_id: &str) -> Option<&GameServer> {
        self.servers.iter().find(|s| s.id == server_id)
    }

    #[cfg(test)]
    pub fn get_world(&self, world_id: &str) -> Option<(&GameServer, &WorldServer)> {
        for server in &self.servers {
            if let Some(world) = server.worlds.iter().find(|w| w.id == world_id) {
                return Some((server, world));
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_config() {
        let toml_content = r#"
[[servers]]
id = "server-1"
name = "Alpha Server"
description = "Main game server"

[[servers.worlds]]
id = "world-1-lorencia"
name = "Lorencia"
ip = "127.0.0.1"
port = 55901
max_players = 100
        "#;

        let config: ServerConfig = toml::from_str(toml_content).unwrap();
        assert_eq!(config.servers.len(), 1);
        assert_eq!(config.servers[0].id, "server-1");
        assert_eq!(config.servers[0].worlds.len(), 1);
        assert_eq!(config.servers[0].worlds[0].name, "Lorencia");
    }

    #[test]
    fn test_get_server() {
        let toml_content = r#"
[[servers]]
id = "server-1"
name = "Alpha Server"
description = "Main game server"

[[servers.worlds]]
id = "world-1-lorencia"
name = "Lorencia"
ip = "127.0.0.1"
port = 55901
max_players = 100
        "#;

        let config: ServerConfig = toml::from_str(toml_content).unwrap();
        assert!(config.get_server("server-1").is_some());
        assert!(config.get_server("non-existent").is_none());
    }

    #[test]
    fn test_get_world() {
        let toml_content = r#"
[[servers]]
id = "server-1"
name = "Alpha Server"
description = "Main game server"

[[servers.worlds]]
id = "world-1-lorencia"
name = "Lorencia"
ip = "127.0.0.1"
port = 55901
max_players = 100
        "#;

        let config: ServerConfig = toml::from_str(toml_content).unwrap();
        assert!(config.get_world("world-1-lorencia").is_some());
        assert!(config.get_world("non-existent").is_none());
    }
}
