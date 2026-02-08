//! Binary codec for MU protocol messages over QUIC.

use crate::channel::{InvalidChannel, QuicChannel, TransportKind};
use crate::message::{
    ClientMessage, PROTOCOL_VERSION, PacketPayload, ProtocolVersion, ServerMessage, WirePacket,
};

const STREAM_MAGIC: [u8; 2] = *b"MU";
const STREAM_LENGTH_LEN: usize = 4;
const STREAM_CHANNEL_LEN: usize = 1;
const STREAM_MAGIC_LEN: usize = 2;
const DATAGRAM_CHANNEL_LEN: usize = 1;

/// Number of bytes in the stream frame header.
pub const STREAM_FRAME_HEADER_LEN: usize =
    STREAM_MAGIC_LEN + STREAM_CHANNEL_LEN + STREAM_LENGTH_LEN;

/// Limits used by the wire codec to protect against malformed payloads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CodecLimits {
    pub max_datagram_size: usize,
    pub max_stream_payload_size: usize,
}

impl Default for CodecLimits {
    fn default() -> Self {
        Self {
            // Safe baseline for internet paths without MTU discovery.
            max_datagram_size: 1200,
            max_stream_payload_size: 64 * 1024,
        }
    }
}

/// Decoded datagram frame payload.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecodedDatagramFrame {
    pub channel: QuicChannel,
    pub packet: WirePacket,
}

/// Decoded stream frame payload.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecodedStreamFrame {
    pub channel: QuicChannel,
    pub packet: WirePacket,
}

/// Errors produced while encoding/decoding wire packets.
#[derive(thiserror::Error, Debug)]
pub enum CodecError {
    #[error("packet version mismatch: expected {expected:?}, got {actual:?}")]
    VersionMismatch {
        expected: ProtocolVersion,
        actual: ProtocolVersion,
    },

    #[error("channel mismatch: frame={channel:?}, payload expects {expected:?}")]
    ChannelMismatch {
        channel: QuicChannel,
        expected: QuicChannel,
    },

    #[error("channel {channel:?} is not valid for datagram transport")]
    NotDatagramChannel { channel: QuicChannel },

    #[error("channel {channel:?} is not valid for stream transport")]
    NotStreamChannel { channel: QuicChannel },

    #[error("datagram frame is empty")]
    EmptyDatagram,

    #[error("datagram exceeds limit: limit={limit} actual={actual}")]
    DatagramTooLarge { limit: usize, actual: usize },

    #[error("stream payload exceeds limit: limit={limit} actual={actual}")]
    StreamPayloadTooLarge { limit: usize, actual: usize },

    #[error("invalid stream magic: expected [4D,55], got {actual:02X?}")]
    InvalidStreamMagic { actual: [u8; 2] },

    #[error("serialization error: {0}")]
    Serialization(#[from] postcard::Error),

    #[error(transparent)]
    InvalidChannel(#[from] InvalidChannel),
}

/// Wire codec that serializes protocol packets with `postcard` and QUIC-aware framing.
#[derive(Clone, Debug)]
pub struct WireCodec {
    expected_version: ProtocolVersion,
    limits: CodecLimits,
}

impl Default for WireCodec {
    fn default() -> Self {
        Self {
            expected_version: PROTOCOL_VERSION,
            limits: CodecLimits::default(),
        }
    }
}

impl WireCodec {
    #[must_use]
    pub const fn new(expected_version: ProtocolVersion, limits: CodecLimits) -> Self {
        Self {
            expected_version,
            limits,
        }
    }

    #[must_use]
    pub const fn expected_version(&self) -> ProtocolVersion {
        self.expected_version
    }

    #[must_use]
    pub const fn limits(&self) -> CodecLimits {
        self.limits
    }

    /// Encodes a datagram frame. The first byte is the `QuicChannel` id.
    pub fn encode_datagram_frame(
        &self,
        channel: QuicChannel,
        packet: &WirePacket,
    ) -> Result<Vec<u8>, CodecError> {
        if channel.transport() != TransportKind::Datagram {
            return Err(CodecError::NotDatagramChannel { channel });
        }
        self.validate_version(packet)?;
        self.validate_channel(channel, packet)?;

        let payload = postcard::to_stdvec(packet)?;
        let frame_len = DATAGRAM_CHANNEL_LEN + payload.len();
        if frame_len > self.limits.max_datagram_size {
            return Err(CodecError::DatagramTooLarge {
                limit: self.limits.max_datagram_size,
                actual: frame_len,
            });
        }

        let mut frame = Vec::with_capacity(frame_len);
        frame.push(channel as u8);
        frame.extend_from_slice(&payload);
        Ok(frame)
    }

    /// Decodes a datagram frame previously produced by `encode_datagram_frame`.
    pub fn decode_datagram_frame(&self, frame: &[u8]) -> Result<DecodedDatagramFrame, CodecError> {
        if frame.is_empty() {
            return Err(CodecError::EmptyDatagram);
        }
        if frame.len() > self.limits.max_datagram_size {
            return Err(CodecError::DatagramTooLarge {
                limit: self.limits.max_datagram_size,
                actual: frame.len(),
            });
        }

        let channel = QuicChannel::try_from(frame[0])?;
        if channel.transport() != TransportKind::Datagram {
            return Err(CodecError::NotDatagramChannel { channel });
        }

        let packet: WirePacket = postcard::from_bytes(&frame[DATAGRAM_CHANNEL_LEN..])?;
        self.validate_version(&packet)?;
        self.validate_channel(channel, &packet)?;

        Ok(DecodedDatagramFrame { channel, packet })
    }

    /// Encodes a stream frame.
    ///
    /// Frame format:
    /// - bytes 0..2: magic `MU`
    /// - byte 2: channel id
    /// - bytes 3..7: payload length (LE u32)
    /// - remaining bytes: postcard payload
    pub fn encode_stream_frame(
        &self,
        channel: QuicChannel,
        packet: &WirePacket,
    ) -> Result<Vec<u8>, CodecError> {
        if channel.transport() == TransportKind::Datagram {
            return Err(CodecError::NotStreamChannel { channel });
        }
        self.validate_version(packet)?;
        self.validate_channel(channel, packet)?;

        let payload = postcard::to_stdvec(packet)?;
        if payload.len() > self.limits.max_stream_payload_size {
            return Err(CodecError::StreamPayloadTooLarge {
                limit: self.limits.max_stream_payload_size,
                actual: payload.len(),
            });
        }

        let mut frame = Vec::with_capacity(STREAM_FRAME_HEADER_LEN + payload.len());
        frame.extend_from_slice(&STREAM_MAGIC);
        frame.push(channel as u8);
        frame.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        frame.extend_from_slice(&payload);
        Ok(frame)
    }

    /// Attempts to decode a single stream frame from the beginning of `buffer`.
    ///
    /// Returns `Ok(None)` when there are not enough bytes yet.
    pub fn try_decode_stream_frame(
        &self,
        buffer: &[u8],
    ) -> Result<Option<(DecodedStreamFrame, usize)>, CodecError> {
        if buffer.len() < STREAM_FRAME_HEADER_LEN {
            return Ok(None);
        }

        let actual_magic = [buffer[0], buffer[1]];
        if actual_magic != STREAM_MAGIC {
            return Err(CodecError::InvalidStreamMagic {
                actual: actual_magic,
            });
        }

        let channel = QuicChannel::try_from(buffer[2])?;
        if channel.transport() == TransportKind::Datagram {
            return Err(CodecError::NotStreamChannel { channel });
        }

        let payload_len = u32::from_le_bytes([buffer[3], buffer[4], buffer[5], buffer[6]]) as usize;
        if payload_len > self.limits.max_stream_payload_size {
            return Err(CodecError::StreamPayloadTooLarge {
                limit: self.limits.max_stream_payload_size,
                actual: payload_len,
            });
        }

        let total_len = STREAM_FRAME_HEADER_LEN + payload_len;
        if buffer.len() < total_len {
            return Ok(None);
        }

        let packet: WirePacket = postcard::from_bytes(&buffer[STREAM_FRAME_HEADER_LEN..total_len])?;
        self.validate_version(&packet)?;
        self.validate_channel(channel, &packet)?;

        Ok(Some((DecodedStreamFrame { channel, packet }, total_len)))
    }

    fn validate_version(&self, packet: &WirePacket) -> Result<(), CodecError> {
        if packet.version != self.expected_version {
            return Err(CodecError::VersionMismatch {
                expected: self.expected_version,
                actual: packet.version,
            });
        }
        Ok(())
    }

    fn validate_channel(
        &self,
        channel: QuicChannel,
        packet: &WirePacket,
    ) -> Result<(), CodecError> {
        let expected = preferred_channel(&packet.payload);
        if channel != expected {
            return Err(CodecError::ChannelMismatch { channel, expected });
        }
        Ok(())
    }
}

/// Returns the default channel for a payload variant.
#[must_use]
pub fn preferred_channel(payload: &PacketPayload) -> QuicChannel {
    match payload {
        PacketPayload::Client(msg) => match msg {
            ClientMessage::Move(_) => QuicChannel::GameplayInput,
            ClientMessage::UseSkill(_) => QuicChannel::GameplayEvent,
            ClientMessage::Chat(_) => QuicChannel::Chat,
            ClientMessage::Hello(_)
            | ClientMessage::KeepAlive { .. }
            | ClientMessage::SelectCharacter { .. }
            | ClientMessage::MapTransferAck { .. }
            | ClientMessage::Logout => QuicChannel::Control,
        },
        PacketPayload::Server(msg) => match msg {
            ServerMessage::StateDelta { .. } => QuicChannel::GameplayInput,
            ServerMessage::Chat(_) => QuicChannel::Chat,
            ServerMessage::EnterMap { .. } => QuicChannel::GameplayEvent,
            ServerMessage::HelloAck { .. }
            | ServerMessage::CharacterList { .. }
            | ServerMessage::MapTransfer(_)
            | ServerMessage::Pong { .. }
            | ServerMessage::Error { .. } => QuicChannel::Control,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::{ClientMessage, MoveInput, RouteKey, WirePacket};

    fn sample_packet() -> WirePacket {
        WirePacket::client(
            10,
            RouteKey {
                world_id: 1,
                entry_id: 2,
                map_id: 3,
                instance_id: 1,
            },
            7,
            Some(6),
            5_000,
            ClientMessage::Move(MoveInput {
                client_tick: 22,
                x: 125,
                y: 87,
                direction: 3,
                path: [1, 2, 3, 4, 5, 6, 7, 8],
            }),
        )
    }

    #[test]
    fn datagram_roundtrip() {
        let codec = WireCodec::default();
        let packet = sample_packet();

        let frame = codec
            .encode_datagram_frame(QuicChannel::GameplayInput, &packet)
            .unwrap();
        let decoded = codec.decode_datagram_frame(&frame).unwrap();

        assert_eq!(decoded.channel, QuicChannel::GameplayInput);
        assert_eq!(decoded.packet, packet);
    }

    #[test]
    fn stream_decode_waits_for_complete_frame() {
        let codec = WireCodec::default();
        let packet = WirePacket::client(
            10,
            RouteKey::LOBBY,
            1,
            None,
            50,
            ClientMessage::KeepAlive { client_time_ms: 50 },
        );
        let frame = codec
            .encode_stream_frame(QuicChannel::Control, &packet)
            .unwrap();

        let partial = &frame[..frame.len() - 1];
        assert!(codec.try_decode_stream_frame(partial).unwrap().is_none());
    }
}
