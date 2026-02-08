//! Core protocol types shared between the client and server crates.
//!
//! This crate now exposes a versioned protocol model designed for QUIC-based
//! transport.

pub mod channel;
pub mod codec;
pub mod message;

pub use channel::{DeliveryGuarantee, QuicChannel, TransportKind};
pub use codec::{
    CodecError, CodecLimits, DecodedDatagramFrame, DecodedStreamFrame, STREAM_FRAME_HEADER_LEN,
    WireCodec, preferred_channel,
};
pub use message::{
    ChatChannel, ChatPayload, ClientHello, ClientMessage, MapTransferDirective, MoveInput,
    PROTOCOL_VERSION, PacketPayload, ProtocolVersion, RouteKey, ServerErrorKind, ServerMessage,
    UseSkillInput, WireEnvelope, WirePacket,
};

/// Returns the protocol crate version string.
pub fn protocol_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_matches_pkg() {
        assert_eq!(protocol_version(), env!("CARGO_PKG_VERSION"));
    }
}
