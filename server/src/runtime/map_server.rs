use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use protocol::{ChatPayload, MoveInput, RouteKey, UseSkillInput};
use serde::Serialize;
use tokio::sync::{mpsc, Mutex};

use super::directory::WorldDirectory;
use super::message_hub::{HubMessage, MessageHub, MessageScope};
use super::persistence::{CharacterStateSnapshot, PersistenceHandle};

#[derive(Debug, Clone)]
pub struct MapServerConfig {
    pub route: RouteKey,
    pub map_name: String,
    pub soft_player_cap: u32,
    pub player_tick: Duration,
    pub monster_tick: Duration,
}

#[derive(Debug, Clone, Serialize)]
pub struct MapServerStats {
    pub route: RouteKey,
    pub map_name: String,
    pub current_players: u32,
    pub soft_player_cap: u32,
    pub monster_count: u32,
    pub player_ticks: u64,
    pub monster_ticks: u64,
    pub monster_degradation_level: u8,
    pub player_tick_p95_us: u64,
}

impl MapServerStats {
    fn new(config: &MapServerConfig) -> Self {
        Self {
            route: config.route,
            map_name: config.map_name.clone(),
            current_players: 0,
            soft_player_cap: config.soft_player_cap,
            monster_count: 16,
            player_ticks: 0,
            monster_ticks: 0,
            monster_degradation_level: 0,
            player_tick_p95_us: 0,
        }
    }
}

#[derive(Debug)]
enum MapServerCommand {
    Join {
        character_id: u64,
        x: u16,
        y: u16,
    },
    Leave {
        character_id: u64,
    },
    Move {
        character_id: u64,
        input: MoveInput,
    },
    UseSkill {
        character_id: u64,
        input: UseSkillInput,
    },
    LocalChat {
        session_id: u64,
        character_id: u64,
        chat: ChatPayload,
    },
    Shutdown,
}

#[derive(Debug, Clone)]
struct PlayerState {
    character_id: u64,
    x: u16,
    y: u16,
    hp: u16,
    mp: u16,
    last_tick: u32,
}

#[derive(Clone)]
pub struct MapServerHandle {
    tx: mpsc::Sender<MapServerCommand>,
    stats: Arc<Mutex<MapServerStats>>,
}

impl MapServerHandle {
    pub async fn join(
        &self,
        _session_id: u64,
        character_id: u64,
        x: u16,
        y: u16,
    ) -> anyhow::Result<()> {
        self.tx
            .send(MapServerCommand::Join { character_id, x, y })
            .await?;
        Ok(())
    }

    pub async fn leave(&self, character_id: u64) -> anyhow::Result<()> {
        self.tx
            .send(MapServerCommand::Leave { character_id })
            .await?;
        Ok(())
    }

    pub async fn move_player(&self, character_id: u64, input: MoveInput) -> anyhow::Result<()> {
        self.tx
            .send(MapServerCommand::Move {
                character_id,
                input,
            })
            .await?;
        Ok(())
    }

    pub async fn use_skill(&self, character_id: u64, input: UseSkillInput) -> anyhow::Result<()> {
        self.tx
            .send(MapServerCommand::UseSkill {
                character_id,
                input,
            })
            .await?;
        Ok(())
    }

    pub async fn local_chat(
        &self,
        session_id: u64,
        character_id: u64,
        chat: ChatPayload,
    ) -> anyhow::Result<()> {
        self.tx
            .send(MapServerCommand::LocalChat {
                session_id,
                character_id,
                chat,
            })
            .await?;
        Ok(())
    }

    pub async fn shutdown(&self) -> anyhow::Result<()> {
        self.tx.send(MapServerCommand::Shutdown).await?;
        Ok(())
    }

    pub async fn stats(&self) -> MapServerStats {
        self.stats.lock().await.clone()
    }
}

pub fn start_map_server(
    config: MapServerConfig,
    directory: WorldDirectory,
    persistence: PersistenceHandle,
    message_hub: MessageHub,
) -> MapServerHandle {
    let (tx, mut rx) = mpsc::channel::<MapServerCommand>(4096);
    let stats = Arc::new(Mutex::new(MapServerStats::new(&config)));
    let stats_clone = stats.clone();

    tokio::spawn(async move {
        let mut players: HashMap<u64, PlayerState> = HashMap::new();
        let mut player_tick = tokio::time::interval(config.player_tick);
        let mut monster_tick = tokio::time::interval(config.monster_tick);
        let mut last_player_tick_us: Vec<u64> = Vec::new();

        loop {
            tokio::select! {
                cmd = rx.recv() => {
                    match cmd {
                        Some(MapServerCommand::Join { character_id, x, y }) => {
                            players.insert(character_id, PlayerState {
                                character_id,
                                x,
                                y,
                                hp: 100,
                                mp: 100,
                                last_tick: 0,
                            });

                            let count = players.len() as u32;
                            directory.update_route_players(config.route, count);
                            let mut st = stats_clone.lock().await;
                            st.current_players = count;
                        }
                        Some(MapServerCommand::Leave { character_id }) => {
                            if let Some(player) = players.remove(&character_id) {
                                let _ = persistence.flush_character(player.character_id).await;
                            }

                            let count = players.len() as u32;
                            directory.update_route_players(config.route, count);
                            let mut st = stats_clone.lock().await;
                            st.current_players = count;
                        }
                        Some(MapServerCommand::Move { character_id, input }) => {
                            if let Some(player) = players.get_mut(&character_id) {
                                player.x = input.x;
                                player.y = input.y;
                                player.last_tick = input.client_tick;
                            }
                        }
                        Some(MapServerCommand::UseSkill { character_id, input }) => {
                            if let Some(player) = players.get_mut(&character_id) {
                                player.last_tick = input.client_tick;
                                if input.target_entity_id.is_some() {
                                    player.mp = player.mp.saturating_sub(1);
                                }
                            }
                        }
                        Some(MapServerCommand::LocalChat { session_id, character_id, chat }) => {
                            if players.contains_key(&character_id) {
                                let msg = HubMessage {
                                    from_session_id: session_id,
                                    route: config.route,
                                    payload: chat,
                                };
                                let _ = message_hub.publish(MessageScope::LocalMap(config.route), msg);
                            }
                        }
                        Some(MapServerCommand::Shutdown) | None => {
                            for player in players.values() {
                                let _ = persistence
                                    .flush_character(player.character_id)
                                    .await;
                            }
                            break;
                        }
                    }
                }
                _ = player_tick.tick() => {
                    let started = Instant::now();

                    // Player gameplay processing always runs first.
                    for player in players.values() {
                        let _ = persistence
                            .enqueue_non_critical(CharacterStateSnapshot {
                                character_id: player.character_id,
                                route: config.route,
                                x: player.x,
                                y: player.y,
                                hp: player.hp,
                                mp: player.mp,
                                updated_at_ms: player.last_tick as u64,
                            })
                            .await;
                    }

                    let elapsed = started.elapsed().as_micros() as u64;
                    last_player_tick_us.push(elapsed);
                    if last_player_tick_us.len() > 200 {
                        last_player_tick_us.remove(0);
                    }

                    let mut sorted = last_player_tick_us.clone();
                    sorted.sort_unstable();
                    let p95_idx = ((sorted.len().saturating_sub(1)) as f64 * 0.95) as usize;
                    let p95 = sorted.get(p95_idx).copied().unwrap_or(0);

                    let mut st = stats_clone.lock().await;
                    st.player_ticks += 1;
                    st.player_tick_p95_us = p95;
                    st.current_players = players.len() as u32;

                    // Degrade monster AI first when player loop is under pressure.
                    if p95 > config.player_tick.as_micros() as u64 {
                        st.monster_degradation_level = (st.monster_degradation_level + 1).min(4);
                    } else if st.monster_degradation_level > 0 {
                        st.monster_degradation_level -= 1;
                    }
                }
                _ = monster_tick.tick() => {
                    let mut st = stats_clone.lock().await;

                    // Monster updates are intentionally lower priority.
                    let skip_ratio = st.monster_degradation_level as u64;
                    if skip_ratio > 0 && st.monster_ticks % (skip_ratio + 1) != 0 {
                        st.monster_ticks += 1;
                        continue;
                    }

                    st.monster_ticks += 1;
                }
            }
        }
    });

    MapServerHandle { tx, stats }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::{config::RuntimeConfig, directory::WorldDirectory};

    #[tokio::test]
    async fn join_move_and_leave_player() {
        let config = RuntimeConfig::default();
        let directory = WorldDirectory::from_runtime_config(&config);
        let sink = Arc::new(crate::runtime::persistence::InMemoryPersistenceSink::new());
        let persistence = crate::runtime::persistence::start_persistence_worker(
            Duration::from_millis(10),
            Duration::from_millis(10),
            100,
            sink,
        );

        let route = RouteKey {
            world_id: 1,
            entry_id: 1,
            map_id: 0,
            instance_id: 1,
        };

        let map = start_map_server(
            MapServerConfig {
                route,
                map_name: "Lorencia".to_string(),
                soft_player_cap: 300,
                player_tick: Duration::from_millis(10),
                monster_tick: Duration::from_millis(20),
            },
            directory.clone(),
            persistence.clone(),
            MessageHub::default(),
        );

        map.join(10, 99, 10, 10).await.unwrap();
        map.move_player(
            99,
            MoveInput {
                client_tick: 1,
                x: 20,
                y: 30,
                direction: 1,
                path: [1, 2, 3, 4, 5, 6, 7, 8],
            },
        )
        .await
        .unwrap();

        tokio::time::sleep(Duration::from_millis(40)).await;
        assert_eq!(directory.current_players_for_route(route), Some(1));

        map.leave(99).await.unwrap();
        tokio::time::sleep(Duration::from_millis(20)).await;
        assert_eq!(directory.current_players_for_route(route), Some(0));

        map.shutdown().await.unwrap();
        persistence.shutdown().await.unwrap();
    }
}
