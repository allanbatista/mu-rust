//! Versioned protocol messages for MU's QUIC transport.

use serde::{Deserialize, Serialize};

/// Current protocol version expected by client and server.
pub const PROTOCOL_VERSION: ProtocolVersion = ProtocolVersion::new(2, 0);

/// Semantic protocol version used in every wire envelope.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ProtocolVersion {
    pub major: u8,
    pub minor: u8,
}

impl ProtocolVersion {
    #[must_use]
    pub const fn new(major: u8, minor: u8) -> Self {
        Self { major, minor }
    }
}

/// Route metadata for the world/entry/map shard handling this packet.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct RouteKey {
    pub world_id: u16,
    pub entry_id: u16,
    pub map_id: u16,
    pub instance_id: u16,
}

impl RouteKey {
    /// Route used before the character enters a map.
    pub const LOBBY: Self = Self {
        world_id: 0,
        entry_id: 0,
        map_id: 0,
        instance_id: 0,
    };
}

/// Generic envelope with transport-agnostic metadata.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct WireEnvelope<T> {
    pub version: ProtocolVersion,
    pub session_id: u64,
    pub route: RouteKey,
    pub sequence: u32,
    pub ack: Option<u32>,
    pub sent_at_ms: u64,
    pub payload: T,
}

impl<T> WireEnvelope<T> {
    #[must_use]
    pub fn new(
        session_id: u64,
        route: RouteKey,
        sequence: u32,
        ack: Option<u32>,
        sent_at_ms: u64,
        payload: T,
    ) -> Self {
        Self {
            version: PROTOCOL_VERSION,
            session_id,
            route,
            sequence,
            ack,
            sent_at_ms,
            payload,
        }
    }
}

/// In-game chat channels.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ChatChannel {
    Local,
    Whisper,
    Party,
    Guild,
    Global,
}

/// Chat payload shared across client and server messages.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChatPayload {
    pub channel: ChatChannel,
    pub target: Option<String>,
    pub text: String,
}

/// First message sent by the client after transport session setup.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClientHello {
    pub account_id: u64,
    pub auth_token: String,
    pub client_build: String,
    pub locale: String,
}

/// Player movement input.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct MoveInput {
    pub client_tick: u32,
    pub x: u16,
    pub y: u16,
    pub direction: u8,
    pub path: [u8; 8],
}

/// Skill usage request.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct UseSkillInput {
    pub client_tick: u32,
    pub skill_id: u16,
    pub target_entity_id: Option<u32>,
    pub target_x: u16,
    pub target_y: u16,
}

/// Messages produced by the game client.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ClientMessage {
    Hello(ClientHello),
    KeepAlive {
        client_time_ms: u64,
    },
    SelectCharacter {
        character_id: u64,
    },
    Move(MoveInput),
    UseSkill(UseSkillInput),
    Chat(ChatPayload),
    MapTransferAck {
        transfer_id: u64,
        route_token: String,
    },
    Logout,
}

/// Public character data shown at character selection.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CharacterSummary {
    pub character_id: u64,
    pub name: String,
    pub class_id: u8,
    pub level: u16,
}

/// Delta for a single entity in the map snapshot stream.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct EntityDelta {
    pub entity_id: u32,
    pub x: u16,
    pub y: u16,
    pub hp: u16,
    pub state_flags: u16,
}

/// Routing directive used when the player must connect to another map instance.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct MapTransferDirective {
    pub transfer_id: u64,
    pub route: RouteKey,
    pub host: String,
    pub port: u16,
    pub route_token: String,
    pub expires_at_ms: u64,
}

/// Error classes returned by the server.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ServerErrorKind {
    InvalidSession,
    CharacterNotFound,
    RouteUnavailable,
    RateLimited,
    InvalidAction,
    Internal,
}

/// Messages produced by the game server.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ServerMessage {
    HelloAck {
        session_id: u64,
        heartbeat_interval_ms: u32,
        motd: String,
        characters: Vec<CharacterSummary>,
    },
    CharacterList {
        entries: Vec<CharacterSummary>,
    },
    EnterMap {
        entity_id: u32,
        map_id: u16,
        x: u16,
        y: u16,
    },
    StateDelta {
        server_tick: u32,
        entities: Vec<EntityDelta>,
    },
    Chat(ChatPayload),
    MapTransfer(MapTransferDirective),
    Pong {
        server_time_ms: u64,
    },
    Error {
        kind: ServerErrorKind,
        message: String,
    },
}

/// Directional packet payload.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum PacketPayload {
    Client(ClientMessage),
    Server(ServerMessage),
}

/// Standard packet type exchanged over QUIC channels.
pub type WirePacket = WireEnvelope<PacketPayload>;

impl WirePacket {
    #[must_use]
    pub fn client(
        session_id: u64,
        route: RouteKey,
        sequence: u32,
        ack: Option<u32>,
        sent_at_ms: u64,
        payload: ClientMessage,
    ) -> Self {
        WireEnvelope::new(
            session_id,
            route,
            sequence,
            ack,
            sent_at_ms,
            PacketPayload::Client(payload),
        )
    }

    #[must_use]
    pub fn server(
        session_id: u64,
        route: RouteKey,
        sequence: u32,
        ack: Option<u32>,
        sent_at_ms: u64,
        payload: ServerMessage,
    ) -> Self {
        WireEnvelope::new(
            session_id,
            route,
            sequence,
            ack,
            sent_at_ms,
            PacketPayload::Server(payload),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packet_constructors_set_current_version() {
        let packet = WirePacket::client(
            7,
            RouteKey::LOBBY,
            1,
            None,
            99,
            ClientMessage::KeepAlive { client_time_ms: 99 },
        );
        assert_eq!(packet.version, PROTOCOL_VERSION);
    }
}
