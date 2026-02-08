use serde::Deserialize;
use std::fs;
use std::path::Path;
use std::time::Duration;

#[derive(Debug, Clone, Deserialize)]
pub struct RuntimeConfig {
    pub gateway: GatewayConfig,
    pub ticks: TickConfig,
    pub persistence: PersistenceConfig,
    pub worlds: Vec<WorldConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GatewayConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TickConfig {
    pub player_tick_ms: u64,
    pub monster_tick_ms: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PersistenceConfig {
    pub flush_tick_ms: u64,
    pub max_flush_lag_ms: u64,
    pub max_batch_size: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WorldConfig {
    pub id: u16,
    pub name: String,
    pub entry_points: Vec<EntryPointConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EntryPointConfig {
    pub id: u16,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub max_players: u32,
    pub maps: Vec<MapConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MapConfig {
    pub id: u16,
    pub name: String,
    pub base_instances: u16,
    pub soft_player_cap: u32,
}

impl RuntimeConfig {
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let content = fs::read_to_string(path)?;
        let parsed = toml::from_str::<Self>(&content)?;
        Ok(parsed)
    }

    pub fn player_tick(&self) -> Duration {
        Duration::from_millis(self.ticks.player_tick_ms)
    }

    pub fn monster_tick(&self) -> Duration {
        Duration::from_millis(self.ticks.monster_tick_ms)
    }

    pub fn flush_tick(&self) -> Duration {
        Duration::from_millis(self.persistence.flush_tick_ms)
    }

    pub fn max_flush_lag(&self) -> Duration {
        Duration::from_millis(self.persistence.max_flush_lag_ms)
    }
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            gateway: GatewayConfig {
                host: "0.0.0.0".to_string(),
                port: 6000,
            },
            ticks: TickConfig {
                player_tick_ms: 50,
                monster_tick_ms: 150,
            },
            persistence: PersistenceConfig {
                flush_tick_ms: 2_000,
                max_flush_lag_ms: 15_000,
                max_batch_size: 300,
            },
            worlds: vec![WorldConfig {
                id: 1,
                name: "Midgard".to_string(),
                entry_points: vec![EntryPointConfig {
                    id: 1,
                    name: "Midgard-1".to_string(),
                    host: "127.0.0.1".to_string(),
                    port: 55901,
                    max_players: 5_000,
                    maps: vec![
                        MapConfig {
                            id: 0,
                            name: "Lorencia".to_string(),
                            base_instances: 1,
                            soft_player_cap: 300,
                        },
                        MapConfig {
                            id: 1,
                            name: "Noria".to_string(),
                            base_instances: 1,
                            soft_player_cap: 300,
                        },
                    ],
                }],
            }],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_valid() {
        let config = RuntimeConfig::default();
        assert_eq!(config.worlds.len(), 1);
        assert!(!config.worlds[0].entry_points.is_empty());
        assert!(config.player_tick().as_millis() > 0);
    }

    #[test]
    fn parse_toml_runtime_config() {
        let toml = r#"
[gateway]
host = "0.0.0.0"
port = 6000

[ticks]
player_tick_ms = 50
monster_tick_ms = 200

[persistence]
flush_tick_ms = 2000
max_flush_lag_ms = 15000
max_batch_size = 200

[[worlds]]
id = 1
name = "Midgard"

[[worlds.entry_points]]
id = 1
name = "Midgard-1"
host = "127.0.0.1"
port = 55901
max_players = 1000

[[worlds.entry_points.maps]]
id = 0
name = "Lorencia"
base_instances = 1
soft_player_cap = 300
"#;

        let config: RuntimeConfig = toml::from_str(toml).expect("valid runtime config");
        assert_eq!(config.gateway.port, 6000);
        assert_eq!(config.worlds[0].entry_points[0].maps[0].name, "Lorencia");
    }
}
