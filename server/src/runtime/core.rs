use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use dashmap::DashMap;
use protocol::message::{CharacterSummary, EntityDelta};
use protocol::{
    ClientHello, ClientMessage, MapTransferDirective, PacketPayload, RouteKey, ServerErrorKind,
    ServerMessage, WireCodec, WirePacket,
};
use serde::Serialize;
use tokio::sync::Mutex as AsyncMutex;

use super::config::RuntimeConfig;
use super::directory::{MapRoute, WorldDirectory, WorldDirectorySnapshot};
use super::map_server::{start_map_server, MapServerConfig, MapServerHandle};
use super::message_hub::MessageHub;
use super::persistence::{
    start_persistence_worker, CriticalEvent, CriticalEventKind, InMemoryPersistenceSink,
    PersistenceHandle,
};
use crate::auth_token::{AuthSessionClaims, AuthTokenService};
use crate::protocol_runtime::{IngressPacket, ProtocolRuntime, ProtocolRuntimeError};

#[derive(Debug, Clone)]
struct PendingTransfer {
    session_id: u64,
    transfer_id: u64,
    character_id: u64,
    route: RouteKey,
}

#[derive(Debug, Clone)]
struct AuthenticatedSession {
    account_id: u64,
    expires_at_ms: u64,
    characters: HashMap<u64, CharacterSummary>,
}

impl AuthenticatedSession {
    fn from_claims(claims: AuthSessionClaims) -> Self {
        let characters = claims
            .characters
            .into_iter()
            .map(|entry| {
                let summary = entry.into_protocol();
                (summary.character_id, summary)
            })
            .collect::<HashMap<_, _>>();

        Self {
            account_id: claims.account_id,
            expires_at_ms: claims.expires_at_ms,
            characters,
        }
    }

    fn character_list(&self) -> Vec<CharacterSummary> {
        let mut list: Vec<CharacterSummary> = self.characters.values().cloned().collect();
        list.sort_by_key(|entry| entry.character_id);
        list
    }

    fn has_character(&self, character_id: u64) -> bool {
        self.characters.contains_key(&character_id)
    }

    fn is_expired(&self, now_ms: u64) -> bool {
        now_ms >= self.expires_at_ms
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct RuntimeStats {
    pub online_maps: usize,
    pub active_transfers: usize,
    pub active_sessions_in_maps: usize,
}

#[derive(Clone)]
pub struct MuCoreRuntime {
    config: RuntimeConfig,
    directory: WorldDirectory,
    message_hub: MessageHub,
    persistence: PersistenceHandle,
    map_servers: Arc<DashMap<RouteKey, MapServerHandle>>,
    protocol_runtime: ProtocolRuntime,
    auth_tokens: AuthTokenService,
    authenticated_sessions: Arc<DashMap<u64, AuthenticatedSession>>,
    transfer_seq: Arc<AtomicU64>,
    pending_transfers: Arc<DashMap<u64, PendingTransfer>>,
    session_routes: Arc<DashMap<u64, (u64, RouteKey)>>,
    scale_lock: Arc<AsyncMutex<()>>,
}

impl MuCoreRuntime {
    pub fn bootstrap(config: RuntimeConfig, auth_tokens: AuthTokenService) -> anyhow::Result<Self> {
        let directory = WorldDirectory::from_runtime_config(&config);
        let message_hub = MessageHub::default();

        let sink = Arc::new(InMemoryPersistenceSink::new());
        let persistence = start_persistence_worker(
            config.flush_tick(),
            config.max_flush_lag(),
            config.persistence.max_batch_size,
            sink,
        );

        let map_servers = Arc::new(DashMap::new());
        for world in &config.worlds {
            for entry in &world.entry_points {
                for map in &entry.maps {
                    for instance_id in 1..=map.base_instances {
                        let route = RouteKey {
                            world_id: world.id,
                            entry_id: entry.id,
                            map_id: map.id,
                            instance_id,
                        };

                        let handle = start_map_server(
                            MapServerConfig {
                                route,
                                map_name: map.name.clone(),
                                soft_player_cap: map.soft_player_cap,
                                player_tick: config.player_tick(),
                                monster_tick: config.monster_tick(),
                            },
                            directory.clone(),
                            persistence.clone(),
                            message_hub.clone(),
                        );

                        map_servers.insert(route, handle);
                    }
                }
            }
        }

        let protocol_runtime = ProtocolRuntime::new(WireCodec::default(), "Welcome to MU Online");

        Ok(Self {
            config,
            directory,
            message_hub,
            persistence,
            map_servers,
            protocol_runtime,
            auth_tokens,
            authenticated_sessions: Arc::new(DashMap::new()),
            transfer_seq: Arc::new(AtomicU64::new(1)),
            pending_transfers: Arc::new(DashMap::new()),
            session_routes: Arc::new(DashMap::new()),
            scale_lock: Arc::new(AsyncMutex::new(())),
        })
    }

    pub fn directory_snapshot(&self) -> WorldDirectorySnapshot {
        self.directory.snapshot()
    }

    pub async fn runtime_stats(&self) -> RuntimeStats {
        RuntimeStats {
            online_maps: self.map_servers.len(),
            active_transfers: self.pending_transfers.len(),
            active_sessions_in_maps: self.session_routes.len(),
        }
    }

    pub async fn handle_datagram_frame(
        &self,
        datagram: &[u8],
        server_time_ms: u64,
    ) -> Result<Option<WirePacket>, ProtocolRuntimeError> {
        let ingress = self.protocol_runtime.decode_v2_datagram(datagram)?;
        self.dispatch_ingress_packet(ingress, server_time_ms).await
    }

    pub async fn handle_stream_bytes(
        &self,
        bytes: &[u8],
        server_time_ms: u64,
    ) -> Result<Vec<WirePacket>, ProtocolRuntimeError> {
        let (frames, consumed) = self.protocol_runtime.decode_v2_stream_batch(bytes)?;

        if consumed < bytes.len() {
            log::debug!(
                "QUIC stream payload contains trailing bytes (consumed={} total={})",
                consumed,
                bytes.len()
            );
        }

        let mut responses = Vec::new();
        for ingress in frames {
            if let Some(packet) = self
                .dispatch_ingress_packet(ingress, server_time_ms)
                .await?
            {
                responses.push(packet);
            }
        }

        Ok(responses)
    }

    async fn dispatch_ingress_packet(
        &self,
        ingress: IngressPacket,
        server_time_ms: u64,
    ) -> Result<Option<WirePacket>, ProtocolRuntimeError> {
        match ingress {
            IngressPacket::V2Datagram(frame) => {
                self.handle_client_packet(frame.packet, server_time_ms)
                    .await
            }
            IngressPacket::V2Stream(frame) => {
                self.handle_client_packet(frame.packet, server_time_ms)
                    .await
            }
        }
    }

    pub async fn handle_client_packet(
        &self,
        packet: WirePacket,
        server_time_ms: u64,
    ) -> Result<Option<WirePacket>, ProtocolRuntimeError> {
        let client_message = match &packet.payload {
            PacketPayload::Client(message) => message,
            PacketPayload::Server(_) => return Err(ProtocolRuntimeError::UnexpectedPacketDirection),
        };

        if let ClientMessage::Hello(hello) = client_message {
            return Ok(Some(self.handle_hello(&packet, hello, server_time_ms)));
        }

        let auth_session = match self.authenticated_session(packet.session_id, server_time_ms) {
            Some(session) => session,
            None => {
                return Ok(Some(self.error_for_request(
                    &packet,
                    server_time_ms,
                    ServerErrorKind::InvalidSession,
                    "Session is not authenticated",
                )))
            }
        };

        let baseline = self
            .protocol_runtime
            .baseline_response(&packet, server_time_ms)?;

        match client_message {
            ClientMessage::SelectCharacter { character_id } => {
                if !auth_session.has_character(*character_id) {
                    return Ok(Some(self.error_for_request(
                        &packet,
                        server_time_ms,
                        ServerErrorKind::CharacterNotFound,
                        "Character does not belong to authenticated session",
                    )));
                }

                let response = self
                    .handle_select_character(packet.session_id, *character_id, server_time_ms)
                    .await;
                return Ok(Some(response));
            }
            ClientMessage::MapTransferAck { transfer_id } => {
                let response = self
                    .handle_transfer_ack(packet.session_id, *transfer_id, server_time_ms)
                    .await;
                return Ok(Some(response));
            }
            ClientMessage::Move(input) => {
                let character_id = match self.character_for_session(packet.session_id) {
                    Some(character_id) => character_id,
                    None => {
                        return Ok(Some(self.error_for_request(
                            &packet,
                            server_time_ms,
                            ServerErrorKind::InvalidAction,
                            "Character must enter a map before moving",
                        )))
                    }
                };

                let map = self
                    .map_servers
                    .get(&packet.route)
                    .map(|entry| entry.value().clone());

                if let Some(map) = map {
                    let _ = map.move_player(character_id, input.clone()).await;
                } else {
                    return Ok(Some(self.error_for_request(
                        &packet,
                        server_time_ms,
                        ServerErrorKind::RouteUnavailable,
                        "Map route is not available",
                    )));
                }

                return Ok(Some(self.response_for_request(
                    &packet,
                    server_time_ms,
                    ServerMessage::StateDelta {
                        server_tick: input.client_tick,
                        entities: vec![EntityDelta {
                            entity_id: character_id as u32,
                            x: input.x,
                            y: input.y,
                            hp: 100,
                            state_flags: 0,
                        }],
                    },
                )));
            }
            ClientMessage::UseSkill(input) => {
                let map = self
                    .map_servers
                    .get(&packet.route)
                    .map(|entry| entry.value().clone());

                if let Some(map) = map {
                    let character_id =
                        self.character_for_session(packet.session_id).unwrap_or(0);
                    let _ = map.use_skill(character_id, input.clone()).await;

                    // Critical operations should be persisted immediately.
                    if input.skill_id >= 200 {
                        let _ = self
                            .persistence
                            .record_critical(CriticalEvent {
                                event_id: ((packet.session_id as u128) << 64)
                                    | packet.sequence as u128,
                                character_id,
                                route: packet.route,
                                kind: CriticalEventKind::EconomyMutation,
                                payload: format!("skill:{}", input.skill_id),
                                occurred_at_ms: server_time_ms,
                            })
                            .await;
                    }
                }
            }
            ClientMessage::Chat(chat) => {
                let map = self
                    .map_servers
                    .get(&packet.route)
                    .map(|entry| entry.value().clone());

                if let Some(map) = map {
                    let character_id =
                        self.character_for_session(packet.session_id).unwrap_or(0);
                    let _ = map
                        .local_chat(packet.session_id, character_id, chat.clone())
                        .await;
                }
            }
            ClientMessage::Logout => {
                self.detach_session_from_map(packet.session_id).await;
                self.authenticated_sessions.remove(&packet.session_id);
            }
            ClientMessage::Hello(_) | ClientMessage::KeepAlive { .. } => {}
        }

        Ok(baseline)
    }

    pub async fn shutdown(&self) -> anyhow::Result<()> {
        let handles: Vec<_> = self
            .map_servers
            .iter()
            .map(|entry| entry.value().clone())
            .collect();

        for map in handles {
            map.shutdown().await?;
        }

        self.persistence.shutdown().await?;
        Ok(())
    }

    pub async fn map_stats(&self) -> Vec<super::map_server::MapServerStats> {
        let handles: Vec<_> = self
            .map_servers
            .iter()
            .map(|entry| entry.value().clone())
            .collect();

        let mut out = Vec::new();
        for map in handles {
            out.push(map.stats().await);
        }

        out.sort_by_key(|s| {
            (
                s.route.world_id,
                s.route.entry_id,
                s.route.map_id,
                s.route.instance_id,
            )
        });

        out
    }

    pub async fn persistence_metrics(&self) -> super::persistence::PersistenceMetrics {
        self.persistence.metrics().await
    }

    pub fn config(&self) -> &RuntimeConfig {
        &self.config
    }

    pub fn message_hub(&self) -> &MessageHub {
        &self.message_hub
    }

    fn handle_hello(
        &self,
        packet: &WirePacket,
        hello: &ClientHello,
        server_time_ms: u64,
    ) -> WirePacket {
        let claims = match self
            .auth_tokens
            .verify(&hello.auth_token, server_time_ms)
        {
            Ok(claims) => claims,
            Err(err) => {
                log::warn!("Failed QUIC hello authentication: {}", err);
                return self.error_for_request(
                    packet,
                    server_time_ms,
                    ServerErrorKind::InvalidSession,
                    "Invalid auth token",
                );
            }
        };

        if claims.account_id != hello.account_id {
            log::warn!(
                "Failed QUIC hello authentication: account mismatch (packet={} claims={})",
                hello.account_id,
                claims.account_id
            );
            return self.error_for_request(
                packet,
                server_time_ms,
                ServerErrorKind::InvalidSession,
                "Account mismatch in auth token",
            );
        }

        let auth_session = AuthenticatedSession::from_claims(claims);
        let characters = auth_session.character_list();
        self.authenticated_sessions
            .insert(packet.session_id, auth_session.clone());

        log::info!(
            "QUIC session authenticated: session_id={} account_id={} characters={}",
            packet.session_id,
            auth_session.account_id,
            characters.len()
        );

        self.response_for_request(
            packet,
            server_time_ms,
            ServerMessage::HelloAck {
                session_id: packet.session_id,
                heartbeat_interval_ms: 5_000,
                motd: "Welcome to MU Online".to_string(),
                characters,
            },
        )
    }

    fn authenticated_session(
        &self,
        session_id: u64,
        server_time_ms: u64,
    ) -> Option<AuthenticatedSession> {
        let session = self
            .authenticated_sessions
            .get(&session_id)
            .map(|entry| entry.value().clone())?;

        if session.is_expired(server_time_ms) {
            self.authenticated_sessions.remove(&session_id);
            return None;
        }

        Some(session)
    }

    fn response_for_request(
        &self,
        request: &WirePacket,
        server_time_ms: u64,
        message: ServerMessage,
    ) -> WirePacket {
        WirePacket::server(
            request.session_id,
            request.route,
            request.sequence.wrapping_add(1),
            Some(request.sequence),
            server_time_ms,
            message,
        )
    }

    fn error_for_request(
        &self,
        request: &WirePacket,
        server_time_ms: u64,
        kind: ServerErrorKind,
        message: &str,
    ) -> WirePacket {
        self.response_for_request(
            request,
            server_time_ms,
            ServerMessage::Error {
                kind,
                message: message.to_string(),
            },
        )
    }

    fn error_for_unbound_session(
        &self,
        session_id: u64,
        sequence: u32,
        server_time_ms: u64,
        kind: ServerErrorKind,
        message: &str,
    ) -> WirePacket {
        WirePacket::server(
            session_id,
            RouteKey::LOBBY,
            sequence,
            None,
            server_time_ms,
            ServerMessage::Error {
                kind,
                message: message.to_string(),
            },
        )
    }

    async fn handle_select_character(
        &self,
        session_id: u64,
        character_id: u64,
        server_time_ms: u64,
    ) -> WirePacket {
        // For MVP we route character selection to map_id 0 in the least loaded entry.
        let target_world = self.config.worlds.first().map(|w| w.id).unwrap_or(0);

        let entry = self.directory.select_best_entry(target_world);
        let map_route = match entry.as_ref() {
            Some(entry) => {
                self.resolve_or_scale_map_route(entry.world_id, entry.entry_id, 0)
                    .await
            }
            None => None,
        };

        match (entry, map_route) {
            (Some(entry), Some(map)) => {
                let transfer_id = self.transfer_seq.fetch_add(1, Ordering::Relaxed);
                self.pending_transfers.insert(
                    transfer_id,
                    PendingTransfer {
                        session_id,
                        transfer_id,
                        character_id,
                        route: map.route,
                    },
                );

                WirePacket::server(
                    session_id,
                    RouteKey::LOBBY,
                    transfer_id as u32,
                    None,
                    server_time_ms,
                    ServerMessage::MapTransfer(MapTransferDirective {
                        transfer_id,
                        route: map.route,
                        host: entry.host,
                        port: entry.port,
                        route_token: format!("rt-{session_id}-{transfer_id}"),
                        expires_at_ms: server_time_ms + 30_000,
                    }),
                )
            }
            _ => WirePacket::server(
                session_id,
                RouteKey::LOBBY,
                0,
                None,
                server_time_ms,
                ServerMessage::Error {
                    kind: ServerErrorKind::RouteUnavailable,
                    message: "No route available".to_string(),
                },
            ),
        }
    }

    async fn resolve_or_scale_map_route(
        &self,
        world_id: u16,
        entry_id: u16,
        map_id: u16,
    ) -> Option<MapRoute> {
        if let Some(route) = self
            .directory
            .select_best_map_instance(world_id, entry_id, map_id)
        {
            return Some(route);
        }

        // Serialize scale-out decisions to avoid creating duplicate instances.
        let _guard = self.scale_lock.lock().await;

        if let Some(route) = self
            .directory
            .select_best_map_instance(world_id, entry_id, map_id)
        {
            return Some(route);
        }

        self.spawn_additional_map_instance(world_id, entry_id, map_id)
            .await?;

        self.directory
            .select_best_map_instance(world_id, entry_id, map_id)
    }

    async fn spawn_additional_map_instance(
        &self,
        world_id: u16,
        entry_id: u16,
        map_id: u16,
    ) -> Option<()> {
        let (map_name, soft_player_cap) =
            self.directory.map_template(world_id, entry_id, map_id)?;
        let instance_id = self
            .directory
            .next_instance_id(world_id, entry_id, map_id)?;

        let route = RouteKey {
            world_id,
            entry_id,
            map_id,
            instance_id,
        };

        if !self.directory.register_instance_route(route) {
            return Some(());
        }

        let handle = start_map_server(
            MapServerConfig {
                route,
                map_name,
                soft_player_cap,
                player_tick: self.config.player_tick(),
                monster_tick: self.config.monster_tick(),
            },
            self.directory.clone(),
            self.persistence.clone(),
            self.message_hub.clone(),
        );

        self.map_servers.insert(route, handle);

        log::info!(
            "Spawned dynamic map instance world={} entry={} map={} instance={}",
            world_id,
            entry_id,
            map_id,
            instance_id
        );

        Some(())
    }

    async fn handle_transfer_ack(
        &self,
        session_id: u64,
        transfer_id: u64,
        server_time_ms: u64,
    ) -> WirePacket {
        let transfer = self
            .pending_transfers
            .remove(&transfer_id)
            .map(|(_, val)| val);
        match transfer {
            Some(transfer) => {
                if transfer.session_id != session_id {
                    return self.error_for_unbound_session(
                        session_id,
                        transfer_id as u32,
                        server_time_ms,
                        ServerErrorKind::InvalidSession,
                        "Transfer does not belong to this session",
                    );
                }

                let map = self
                    .map_servers
                    .get(&transfer.route)
                    .map(|entry| entry.value().clone());

                if let Some(map) = map {
                    let _ = map.join(session_id, transfer.character_id, 125, 125).await;
                    self.session_routes
                        .insert(session_id, (transfer.character_id, transfer.route));

                    WirePacket::server(
                        session_id,
                        transfer.route,
                        transfer.transfer_id as u32,
                        None,
                        server_time_ms,
                        ServerMessage::EnterMap {
                            entity_id: transfer.character_id as u32,
                            map_id: transfer.route.map_id,
                            x: 125,
                            y: 125,
                        },
                    )
                } else {
                    WirePacket::server(
                        session_id,
                        RouteKey::LOBBY,
                        transfer_id as u32,
                        None,
                        server_time_ms,
                        ServerMessage::Error {
                            kind: ServerErrorKind::RouteUnavailable,
                            message: "Map instance unavailable".to_string(),
                        },
                    )
                }
            }
            None => self.error_for_unbound_session(
                session_id,
                transfer_id as u32,
                server_time_ms,
                ServerErrorKind::InvalidAction,
                "Invalid transfer ack",
            ),
        }
    }

    fn character_for_session(&self, session_id: u64) -> Option<u64> {
        self.session_routes
            .get(&session_id)
            .map(|entry| entry.value().0)
    }

    async fn detach_session_from_map(&self, session_id: u64) {
        if let Some((_, (character_id, route))) = self.session_routes.remove(&session_id) {
            let map = self
                .map_servers
                .get(&route)
                .map(|entry| entry.value().clone());

            if let Some(map) = map {
                let _ = map.leave(character_id).await;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    use crate::auth_token::{AuthCharacterSummary, AuthTokenService};
    use protocol::{ClientHello, QuicChannel};

    fn build_runtime() -> MuCoreRuntime {
        let auth_tokens = AuthTokenService::new(
            b"01234567890123456789012345678901".to_vec(),
            Duration::from_secs(3600),
        )
        .expect("auth tokens");

        MuCoreRuntime::bootstrap(RuntimeConfig::default(), auth_tokens).expect("runtime boot")
    }

    fn build_hello_packet(
        runtime: &MuCoreRuntime,
        session_id: u64,
        account_id: u64,
        character_ids: &[u64],
    ) -> WirePacket {
        let token = runtime
            .auth_tokens
            .issue_session_token(
                account_id,
                format!("session-{session_id}"),
                character_ids
                    .iter()
                    .map(|character_id| AuthCharacterSummary {
                        character_id: *character_id,
                        name: format!("Character-{character_id}"),
                        class_id: 1,
                        level: 150,
                    })
                    .collect(),
                100,
            )
            .expect("issue auth token");

        WirePacket::client(
            session_id,
            RouteKey::LOBBY,
            1,
            None,
            100,
            ClientMessage::Hello(ClientHello {
                account_id,
                auth_token: token,
                client_build: "0.1.0".to_string(),
                locale: "pt-BR".to_string(),
            }),
        )
    }

    #[tokio::test]
    async fn select_character_returns_map_transfer() {
        let runtime = build_runtime();

        let hello = runtime
            .handle_client_packet(build_hello_packet(&runtime, 7, 10, &[42]), 100)
            .await
            .expect("hello packet")
            .expect("hello response");
        assert!(matches!(
            hello.payload,
            PacketPayload::Server(ServerMessage::HelloAck { .. })
        ));

        let response = runtime
            .handle_client_packet(
                WirePacket::client(
                    7,
                    RouteKey::LOBBY,
                    1,
                    None,
                    100,
                    ClientMessage::SelectCharacter { character_id: 42 },
                ),
                100,
            )
            .await
            .expect("handle packet")
            .expect("must respond");

        assert!(matches!(
            response.payload,
            PacketPayload::Server(ServerMessage::MapTransfer(_))
        ));

        runtime.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn transfer_ack_enters_map() {
        let runtime = build_runtime();

        let _ = runtime
            .handle_client_packet(build_hello_packet(&runtime, 9, 11, &[99]), 100)
            .await
            .expect("hello packet")
            .expect("hello response");

        let transfer = runtime
            .handle_client_packet(
                WirePacket::client(
                    9,
                    RouteKey::LOBBY,
                    1,
                    None,
                    100,
                    ClientMessage::SelectCharacter { character_id: 99 },
                ),
                100,
            )
            .await
            .unwrap()
            .unwrap();

        let (transfer_id, _map_route) = match transfer.payload {
            PacketPayload::Server(ServerMessage::MapTransfer(directive)) => {
                (directive.transfer_id, directive.route)
            }
            _ => panic!("expected transfer"),
        };

        let enter = runtime
            .handle_client_packet(
                WirePacket::client(
                    9,
                    RouteKey::LOBBY,
                    2,
                    None,
                    110,
                    ClientMessage::MapTransferAck { transfer_id },
                ),
                110,
            )
            .await
            .unwrap()
            .unwrap();

        assert!(matches!(
            enter.payload,
            PacketPayload::Server(ServerMessage::EnterMap { .. })
        ));

        runtime.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn handle_stream_bytes_decodes_and_replies() {
        let runtime = build_runtime();
        let hello = build_hello_packet(&runtime, 10, 10, &[500]);

        let codec = WireCodec::default();
        let frame = codec
            .encode_stream_frame(QuicChannel::Control, &hello)
            .expect("encode stream frame");

        let responses = runtime
            .handle_stream_bytes(&frame, 200)
            .await
            .expect("dispatch stream");

        assert_eq!(responses.len(), 1);
        assert!(matches!(
            responses[0].payload,
            PacketPayload::Server(ServerMessage::HelloAck { .. })
        ));

        runtime.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn handle_datagram_frame_returns_state_delta_after_entering_map() {
        let runtime = build_runtime();

        let _ = runtime
            .handle_client_packet(build_hello_packet(&runtime, 11, 12, &[700]), 100)
            .await
            .expect("hello packet")
            .expect("hello response");

        let transfer = runtime
            .handle_client_packet(
                WirePacket::client(
                    11,
                    RouteKey::LOBBY,
                    2,
                    None,
                    100,
                    ClientMessage::SelectCharacter { character_id: 700 },
                ),
                100,
            )
            .await
            .unwrap()
            .unwrap();

        let (transfer_id, map_route) = match transfer.payload {
            PacketPayload::Server(ServerMessage::MapTransfer(directive)) => {
                (directive.transfer_id, directive.route)
            }
            _ => panic!("expected transfer"),
        };

        let enter = runtime
            .handle_client_packet(
                WirePacket::client(
                    11,
                    RouteKey::LOBBY,
                    3,
                    None,
                    105,
                    ClientMessage::MapTransferAck { transfer_id },
                ),
                105,
            )
            .await
            .unwrap()
            .unwrap();
        match enter.payload {
            PacketPayload::Server(ServerMessage::EnterMap { .. }) => {}
            _ => panic!("expected enter map"),
        }

        let move_packet = WirePacket::client(
            11,
            map_route,
            4,
            None,
            110,
            ClientMessage::Move(protocol::MoveInput {
                client_tick: 1,
                x: 10,
                y: 10,
                direction: 0,
                path: [0; 8],
            }),
        );

        let codec = WireCodec::default();
        let datagram = codec
            .encode_datagram_frame(QuicChannel::GameplayInput, &move_packet)
            .expect("encode datagram");

        let response = runtime
            .handle_datagram_frame(&datagram, 200)
            .await
            .expect("dispatch datagram");

        assert!(matches!(
            response.map(|packet| packet.payload),
            Some(PacketPayload::Server(ServerMessage::StateDelta { .. }))
        ));

        runtime.shutdown().await.unwrap();
    }
}
