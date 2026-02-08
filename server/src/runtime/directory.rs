use std::collections::HashMap;
use std::sync::Arc;

use dashmap::DashMap;
use protocol::RouteKey;
use serde::Serialize;

use super::config::{EntryPointConfig, RuntimeConfig};

#[derive(Debug, Clone, Serialize)]
pub struct EntryPointRoute {
    pub world_id: u16,
    pub entry_id: u16,
    pub host: String,
    pub port: u16,
    pub max_players: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct MapRoute {
    pub route: RouteKey,
    pub map_name: String,
    pub soft_player_cap: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct WorldDirectorySnapshot {
    pub worlds: Vec<WorldSnapshot>,
}

#[derive(Debug, Clone, Serialize)]
pub struct WorldSnapshot {
    pub world_id: u16,
    pub world_name: String,
    pub entries: Vec<EntrySnapshot>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EntrySnapshot {
    pub entry_id: u16,
    pub entry_name: String,
    pub host: String,
    pub port: u16,
    pub current_players: u32,
    pub max_players: u32,
    pub maps: Vec<MapSnapshot>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MapSnapshot {
    pub map_id: u16,
    pub map_name: String,
    pub instance_id: u16,
    pub current_players: u32,
    pub soft_player_cap: u32,
}

#[derive(Debug, Clone)]
struct StaticEntry {
    world_name: String,
    entry_name: String,
    host: String,
    port: u16,
    max_players: u32,
}

#[derive(Debug, Clone)]
struct StaticMap {
    map_name: String,
    soft_player_cap: u32,
}

#[derive(Clone)]
pub struct WorldDirectory {
    // key: (world, entry)
    entry_static: Arc<HashMap<(u16, u16), StaticEntry>>,
    // key: (world, entry, map)
    map_static: Arc<HashMap<(u16, u16, u16), StaticMap>>,
    // key: route
    route_players: Arc<DashMap<RouteKey, u32>>,
}

impl WorldDirectory {
    pub fn from_runtime_config(config: &RuntimeConfig) -> Self {
        let mut entry_static = HashMap::new();
        let mut map_static = HashMap::new();
        let route_players = Arc::new(DashMap::new());

        for world in &config.worlds {
            for entry in &world.entry_points {
                entry_static.insert(
                    (world.id, entry.id),
                    StaticEntry {
                        world_name: world.name.clone(),
                        entry_name: entry.name.clone(),
                        host: entry.host.clone(),
                        port: entry.port,
                        max_players: entry.max_players,
                    },
                );

                for map in &entry.maps {
                    map_static.insert(
                        (world.id, entry.id, map.id),
                        StaticMap {
                            map_name: map.name.clone(),
                            soft_player_cap: map.soft_player_cap,
                        },
                    );

                    for instance_id in 1..=map.base_instances {
                        route_players.insert(
                            RouteKey {
                                world_id: world.id,
                                entry_id: entry.id,
                                map_id: map.id,
                                instance_id,
                            },
                            0,
                        );
                    }
                }
            }
        }

        Self {
            entry_static: Arc::new(entry_static),
            map_static: Arc::new(map_static),
            route_players,
        }
    }

    pub fn select_best_entry(&self, world_id: u16) -> Option<EntryPointRoute> {
        let mut candidates: Vec<((u16, u16), StaticEntry, u32)> = self
            .entry_static
            .iter()
            .filter(|((w, _), _)| *w == world_id)
            .map(|(key, val)| {
                let current_players = self.current_players_for_entry(key.0, key.1);
                (*key, val.clone(), current_players)
            })
            .collect();

        candidates.sort_by_key(|(_, _, players)| *players);
        let ((_, entry_id), entry, current_players) = candidates.into_iter().next()?;

        if current_players >= entry.max_players {
            return None;
        }

        Some(EntryPointRoute {
            world_id,
            entry_id,
            host: entry.host,
            port: entry.port,
            max_players: entry.max_players,
        })
    }

    pub fn select_best_map_instance(
        &self,
        world_id: u16,
        entry_id: u16,
        map_id: u16,
    ) -> Option<MapRoute> {
        let static_map = self.map_static.get(&(world_id, entry_id, map_id))?;

        let mut candidates: Vec<(RouteKey, u32)> = self
            .route_players
            .iter()
            .filter(|entry| {
                let key = entry.key();
                key.world_id == world_id && key.entry_id == entry_id && key.map_id == map_id
            })
            .map(|entry| (*entry.key(), *entry.value()))
            .collect();

        if candidates.is_empty() {
            return None;
        }

        candidates.sort_by_key(|(_, players)| *players);
        let (route, players) = candidates[0];
        if players >= static_map.soft_player_cap {
            return None;
        }

        Some(MapRoute {
            route,
            map_name: static_map.map_name.clone(),
            soft_player_cap: static_map.soft_player_cap,
        })
    }

    pub fn update_route_players(&self, route: RouteKey, players: u32) {
        self.route_players.insert(route, players);
    }

    pub fn increment_route_players(&self, route: RouteKey) -> Option<u32> {
        let mut entry = self.route_players.get_mut(&route)?;
        *entry += 1;
        Some(*entry)
    }

    pub fn decrement_route_players(&self, route: RouteKey) -> Option<u32> {
        let mut entry = self.route_players.get_mut(&route)?;
        if *entry > 0 {
            *entry -= 1;
        }
        Some(*entry)
    }

    pub fn current_players_for_route(&self, route: RouteKey) -> Option<u32> {
        self.route_players.get(&route).map(|e| *e)
    }

    pub fn map_template(&self, world_id: u16, entry_id: u16, map_id: u16) -> Option<(String, u32)> {
        self.map_static
            .get(&(world_id, entry_id, map_id))
            .map(|meta| (meta.map_name.clone(), meta.soft_player_cap))
    }

    pub fn next_instance_id(&self, world_id: u16, entry_id: u16, map_id: u16) -> Option<u16> {
        if !self.map_static.contains_key(&(world_id, entry_id, map_id)) {
            return None;
        }

        let mut highest_instance = 0u16;
        for route_entry in self.route_players.iter() {
            let route = route_entry.key();
            if route.world_id == world_id && route.entry_id == entry_id && route.map_id == map_id {
                highest_instance = highest_instance.max(route.instance_id);
            }
        }

        Some(highest_instance.saturating_add(1).max(1))
    }

    pub fn register_instance_route(&self, route: RouteKey) -> bool {
        if !self
            .map_static
            .contains_key(&(route.world_id, route.entry_id, route.map_id))
        {
            return false;
        }

        self.route_players.insert(route, 0).is_none()
    }

    pub fn snapshot(&self) -> WorldDirectorySnapshot {
        let mut grouped: HashMap<u16, WorldSnapshot> = HashMap::new();

        for ((world_id, entry_id), entry_meta) in self.entry_static.iter() {
            let mut maps = Vec::new();
            for ((w, e, map_id), map_meta) in self.map_static.iter() {
                if *w != *world_id || *e != *entry_id {
                    continue;
                }

                for route_entry in self.route_players.iter() {
                    let route = route_entry.key();
                    if route.world_id == *world_id
                        && route.entry_id == *entry_id
                        && route.map_id == *map_id
                    {
                        maps.push(MapSnapshot {
                            map_id: *map_id,
                            map_name: map_meta.map_name.clone(),
                            instance_id: route.instance_id,
                            current_players: *route_entry.value(),
                            soft_player_cap: map_meta.soft_player_cap,
                        });
                    }
                }
            }

            maps.sort_by_key(|m| (m.map_id, m.instance_id));

            let world_snapshot = grouped.entry(*world_id).or_insert_with(|| WorldSnapshot {
                world_id: *world_id,
                world_name: entry_meta.world_name.clone(),
                entries: Vec::new(),
            });

            world_snapshot.entries.push(EntrySnapshot {
                entry_id: *entry_id,
                entry_name: entry_meta.entry_name.clone(),
                host: entry_meta.host.clone(),
                port: entry_meta.port,
                current_players: self.current_players_for_entry(*world_id, *entry_id),
                max_players: entry_meta.max_players,
                maps,
            });
        }

        let mut worlds: Vec<_> = grouped.into_values().collect();
        worlds.sort_by_key(|w| w.world_id);
        for world in &mut worlds {
            world.entries.sort_by_key(|e| e.entry_id);
        }

        WorldDirectorySnapshot { worlds }
    }

    fn current_players_for_entry(&self, world_id: u16, entry_id: u16) -> u32 {
        self.route_players
            .iter()
            .filter(|entry| {
                let key = entry.key();
                key.world_id == world_id && key.entry_id == entry_id
            })
            .map(|entry| *entry.value())
            .sum()
    }

    pub fn all_entry_points(&self) -> Vec<EntryPointRoute> {
        let mut points = Vec::new();
        for ((world_id, entry_id), entry) in self.entry_static.iter() {
            points.push(EntryPointRoute {
                world_id: *world_id,
                entry_id: *entry_id,
                host: entry.host.clone(),
                port: entry.port,
                max_players: entry.max_players,
            });
        }
        points.sort_by_key(|e| (e.world_id, e.entry_id));
        points
    }

    pub fn entry_route_from_config(world_id: u16, entry: &EntryPointConfig) -> EntryPointRoute {
        EntryPointRoute {
            world_id,
            entry_id: entry.id,
            host: entry.host.clone(),
            port: entry.port,
            max_players: entry.max_players,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::config::RuntimeConfig;

    fn sample_config() -> RuntimeConfig {
        RuntimeConfig::default()
    }

    #[test]
    fn selects_best_map_instance() {
        let config = sample_config();
        let directory = WorldDirectory::from_runtime_config(&config);

        let route = directory
            .select_best_map_instance(1, 1, 0)
            .expect("must find route");
        assert_eq!(route.route.world_id, 1);
        assert_eq!(route.route.entry_id, 1);
    }

    #[test]
    fn snapshot_contains_worlds_entries_and_maps() {
        let config = sample_config();
        let directory = WorldDirectory::from_runtime_config(&config);
        let snapshot = directory.snapshot();

        assert!(!snapshot.worlds.is_empty());
        let world = &snapshot.worlds[0];
        assert_eq!(world.world_id, 1);
        assert!(!world.entries.is_empty());
        assert!(!world.entries[0].maps.is_empty());
    }

    #[test]
    fn players_can_be_incremented_and_decremented() {
        let config = sample_config();
        let directory = WorldDirectory::from_runtime_config(&config);

        let route = RouteKey {
            world_id: 1,
            entry_id: 1,
            map_id: 0,
            instance_id: 1,
        };

        assert_eq!(directory.increment_route_players(route), Some(1));
        assert_eq!(directory.decrement_route_players(route), Some(0));
    }

    #[test]
    fn registers_new_instance_route_for_map_scaling() {
        let config = sample_config();
        let directory = WorldDirectory::from_runtime_config(&config);

        let next_instance = directory
            .next_instance_id(1, 1, 0)
            .expect("map template must exist");

        let new_route = RouteKey {
            world_id: 1,
            entry_id: 1,
            map_id: 0,
            instance_id: next_instance,
        };

        assert!(directory.register_instance_route(new_route));
        assert_eq!(directory.current_players_for_route(new_route), Some(0));
    }
}
