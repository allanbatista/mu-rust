//! Protocol ingress/runtime utilities for protocol v2.
//!
//! This module normalizes incoming packets into the `WirePacket` model.

use protocol::{
    ChatChannel, CodecError, DecodedDatagramFrame, DecodedStreamFrame, PacketPayload,
    ServerMessage, WireCodec, WirePacket,
};

/// Error type returned by protocol runtime operations.
#[derive(thiserror::Error, Debug)]
pub enum ProtocolRuntimeError {
    #[error(transparent)]
    Codec(#[from] CodecError),

    #[error("expected client packet but received server packet")]
    UnexpectedPacketDirection,
}

/// Result of decoding an ingress payload from any supported source.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IngressPacket {
    V2Datagram(DecodedDatagramFrame),
    V2Stream(DecodedStreamFrame),
}

/// Runtime helper for decoding ingress packets and generating baseline responses.
#[derive(Clone, Debug)]
pub struct ProtocolRuntime {
    codec: WireCodec,
    motd: String,
}

impl Default for ProtocolRuntime {
    fn default() -> Self {
        Self {
            codec: WireCodec::default(),
            motd: "Welcome to MU".to_string(),
        }
    }
}

impl ProtocolRuntime {
    #[must_use]
    pub fn new(codec: WireCodec, motd: impl Into<String>) -> Self {
        Self {
            codec,
            motd: motd.into(),
        }
    }

    #[must_use]
    pub fn codec(&self) -> &WireCodec {
        &self.codec
    }

    /// Decodes one v2 datagram ingress frame.
    pub fn decode_v2_datagram(&self, frame: &[u8]) -> Result<IngressPacket, ProtocolRuntimeError> {
        let decoded = self.codec.decode_datagram_frame(frame)?;
        Ok(IngressPacket::V2Datagram(decoded))
    }

    /// Decodes as many complete stream frames as possible from `buffer`.
    ///
    /// Returns `(frames, consumed_bytes)`. The caller should keep
    /// `buffer[consumed_bytes..]` for the next read if it contains a partial frame.
    pub fn decode_v2_stream_batch(
        &self,
        buffer: &[u8],
    ) -> Result<(Vec<IngressPacket>, usize), ProtocolRuntimeError> {
        let mut out = Vec::new();
        let mut consumed = 0;

        loop {
            match self.codec.try_decode_stream_frame(&buffer[consumed..])? {
                Some((frame, used)) => {
                    out.push(IngressPacket::V2Stream(frame));
                    consumed += used;
                }
                None => break,
            }
        }

        Ok((out, consumed))
    }

    /// Minimal baseline response rules for client control/chat packets.
    ///
    /// This keeps response logic centralized while the server migration is in
    /// progress and before full gameplay handlers are plugged in.
    pub fn baseline_response(
        &self,
        packet: &WirePacket,
        server_time_ms: u64,
    ) -> Result<Option<WirePacket>, ProtocolRuntimeError> {
        match &packet.payload {
            PacketPayload::Client(client) => {
                let sequence = packet.sequence.wrapping_add(1);
                let ack = Some(packet.sequence);

                let response = match client {
                    protocol::ClientMessage::Hello(_) => Some(ServerMessage::HelloAck {
                        session_id: packet.session_id,
                        heartbeat_interval_ms: 5_000,
                        motd: self.motd.clone(),
                    }),
                    protocol::ClientMessage::KeepAlive { .. } => {
                        Some(ServerMessage::Pong { server_time_ms })
                    }
                    protocol::ClientMessage::Chat(chat) if chat.channel == ChatChannel::Local => {
                        Some(ServerMessage::Chat(chat.clone()))
                    }
                    _ => None,
                };

                Ok(response.map(|msg| {
                    WirePacket::server(
                        packet.session_id,
                        packet.route,
                        sequence,
                        ack,
                        server_time_ms,
                        msg,
                    )
                }))
            }
            PacketPayload::Server(_) => Err(ProtocolRuntimeError::UnexpectedPacketDirection),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use protocol::{ClientHello, ClientMessage, QuicChannel, RouteKey};

    fn sample_route() -> RouteKey {
        RouteKey {
            world_id: 1,
            entry_id: 1,
            map_id: 0,
            instance_id: 1,
        }
    }

    #[test]
    fn decode_datagram_success() {
        let runtime = ProtocolRuntime::default();
        let packet = WirePacket::client(
            7,
            sample_route(),
            3,
            Some(2),
            900,
            ClientMessage::Move(protocol::MoveInput {
                client_tick: 12,
                x: 100,
                y: 120,
                direction: 2,
                path: [2, 1, 2, 3, 4, 5, 6, 7],
            }),
        );

        let frame = runtime
            .codec()
            .encode_datagram_frame(QuicChannel::GameplayInput, &packet)
            .unwrap();
        let decoded = runtime.decode_v2_datagram(&frame).unwrap();

        match decoded {
            IngressPacket::V2Datagram(frame) => assert_eq!(frame.packet, packet),
            _ => panic!("expected datagram frame"),
        }
    }

    #[test]
    fn decode_stream_batch_consumes_complete_frames() {
        let runtime = ProtocolRuntime::default();

        let hello = WirePacket::client(
            8,
            RouteKey::LOBBY,
            1,
            None,
            1_000,
            ClientMessage::Hello(ClientHello {
                account_id: 11,
                auth_token: "token".into(),
                client_build: "0.1.0".into(),
                locale: "pt-BR".into(),
            }),
        );

        let ping = WirePacket::client(
            8,
            RouteKey::LOBBY,
            2,
            Some(1),
            1_001,
            ClientMessage::KeepAlive {
                client_time_ms: 1_001,
            },
        );

        let mut data = runtime
            .codec()
            .encode_stream_frame(QuicChannel::Control, &hello)
            .unwrap();
        data.extend_from_slice(
            &runtime
                .codec()
                .encode_stream_frame(QuicChannel::Control, &ping)
                .unwrap(),
        );

        let (frames, consumed) = runtime.decode_v2_stream_batch(&data).unwrap();
        assert_eq!(consumed, data.len());
        assert_eq!(frames.len(), 2);
    }

    #[test]
    fn baseline_response_for_keepalive() {
        let runtime = ProtocolRuntime::new(WireCodec::default(), "MOTD");
        let request = WirePacket::client(
            44,
            RouteKey::LOBBY,
            10,
            Some(9),
            2_000,
            ClientMessage::KeepAlive {
                client_time_ms: 2_000,
            },
        );

        let response = runtime
            .baseline_response(&request, 2_500)
            .unwrap()
            .expect("must produce pong");

        assert_eq!(response.session_id, 44);
        assert_eq!(response.sequence, 11);
        assert_eq!(response.ack, Some(10));
        assert!(matches!(
            response.payload,
            PacketPayload::Server(ServerMessage::Pong {
                server_time_ms: 2_500
            })
        ));
    }
}
