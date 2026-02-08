use protocol::channel::QuicChannel;
use protocol::codec::{CodecError, CodecLimits, WireCodec};
use protocol::message::{
    ChatChannel, ChatPayload, ClientHello, ClientMessage, PacketPayload, ProtocolVersion, RouteKey,
    ServerMessage, WirePacket,
};

fn sample_route() -> RouteKey {
    RouteKey {
        world_id: 1,
        entry_id: 2,
        map_id: 3,
        instance_id: 4,
    }
}

fn sample_move_packet() -> WirePacket {
    WirePacket::client(
        100,
        sample_route(),
        10,
        Some(9),
        42_000,
        ClientMessage::Move(protocol::MoveInput {
            client_tick: 120,
            x: 125,
            y: 111,
            direction: 2,
            path: [1, 2, 3, 4, 5, 6, 7, 8],
        }),
    )
}

fn sample_control_packet() -> WirePacket {
    WirePacket::client(
        200,
        RouteKey::LOBBY,
        1,
        None,
        1_000,
        ClientMessage::Hello(ClientHello {
            account_id: 77,
            auth_token: "token-abc".into(),
            client_build: "0.1.0".into(),
            locale: "pt-BR".into(),
        }),
    )
}

fn sample_chat_packet() -> WirePacket {
    WirePacket::server(
        200,
        sample_route(),
        2,
        Some(1),
        1_100,
        ServerMessage::Chat(ChatPayload {
            channel: ChatChannel::Guild,
            target: None,
            text: "evento em devias".into(),
        }),
    )
}

#[test]
fn move_packet_datagram_roundtrip() {
    let codec = WireCodec::default();
    let packet = sample_move_packet();

    let bytes = codec
        .encode_datagram_frame(QuicChannel::GameplayInput, &packet)
        .unwrap();
    let decoded = codec.decode_datagram_frame(&bytes).unwrap();

    assert_eq!(decoded.channel, QuicChannel::GameplayInput);
    assert_eq!(decoded.packet, packet);
}

#[test]
fn control_packet_stream_roundtrip() {
    let codec = WireCodec::default();
    let packet = sample_control_packet();

    let frame = codec
        .encode_stream_frame(QuicChannel::Control, &packet)
        .unwrap();

    let (decoded, consumed) = codec.try_decode_stream_frame(&frame).unwrap().unwrap();
    assert_eq!(consumed, frame.len());
    assert_eq!(decoded.channel, QuicChannel::Control);
    assert_eq!(decoded.packet, packet);
}

#[test]
fn stream_decoder_can_parse_back_to_back_frames() {
    let codec = WireCodec::default();
    let first = codec
        .encode_stream_frame(QuicChannel::Control, &sample_control_packet())
        .unwrap();
    let second = codec
        .encode_stream_frame(QuicChannel::Chat, &sample_chat_packet())
        .unwrap();

    let mut buffer = Vec::new();
    buffer.extend_from_slice(&first);
    buffer.extend_from_slice(&second);

    let (frame1, used1) = codec.try_decode_stream_frame(&buffer).unwrap().unwrap();
    let (frame2, used2) = codec
        .try_decode_stream_frame(&buffer[used1..])
        .unwrap()
        .unwrap();

    assert_eq!(frame1.channel, QuicChannel::Control);
    assert_eq!(frame2.channel, QuicChannel::Chat);
    assert_eq!(used1 + used2, buffer.len());
}

#[test]
fn stream_decoder_returns_none_for_partial_frame() {
    let codec = WireCodec::default();
    let frame = codec
        .encode_stream_frame(QuicChannel::Control, &sample_control_packet())
        .unwrap();

    let partial = &frame[..frame.len() - 3];
    assert!(codec.try_decode_stream_frame(partial).unwrap().is_none());
}

#[test]
fn rejects_channel_payload_mismatch() {
    let codec = WireCodec::default();
    let packet = sample_control_packet();

    let err = codec
        .encode_stream_frame(QuicChannel::Chat, &packet)
        .unwrap_err();

    assert!(matches!(err, CodecError::ChannelMismatch { .. }));
}

#[test]
fn rejects_version_mismatch_during_decode() {
    let compat_codec = WireCodec::new(ProtocolVersion::new(3, 0), CodecLimits::default());
    let mut packet = sample_control_packet();
    packet.version = ProtocolVersion::new(3, 0);

    let frame = compat_codec
        .encode_stream_frame(QuicChannel::Control, &packet)
        .unwrap();

    let err = WireCodec::default()
        .try_decode_stream_frame(&frame)
        .unwrap_err();

    assert!(matches!(err, CodecError::VersionMismatch { .. }));
}

#[test]
fn rejects_oversized_datagram() {
    let tiny_codec = WireCodec::new(
        ProtocolVersion::new(2, 0),
        CodecLimits {
            max_datagram_size: 16,
            max_stream_payload_size: 1024,
        },
    );

    let packet = sample_move_packet();
    let err = tiny_codec
        .encode_datagram_frame(QuicChannel::GameplayInput, &packet)
        .unwrap_err();

    assert!(matches!(err, CodecError::DatagramTooLarge { .. }));
}

#[test]
fn packet_payload_variant_is_preserved() {
    let codec = WireCodec::default();
    let packet = sample_chat_packet();

    let frame = codec
        .encode_stream_frame(QuicChannel::Chat, &packet)
        .unwrap();
    let (decoded, _) = codec.try_decode_stream_frame(&frame).unwrap().unwrap();

    assert!(matches!(
        decoded.packet.payload,
        PacketPayload::Server(ServerMessage::Chat(_))
    ));
}
