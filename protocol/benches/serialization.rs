use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use protocol::channel::QuicChannel;
use protocol::codec::WireCodec;
use protocol::message::{ClientHello, ClientMessage, RouteKey, WirePacket};

fn sample_move_packet() -> WirePacket {
    WirePacket::client(
        42,
        RouteKey {
            world_id: 1,
            entry_id: 1,
            map_id: 0,
            instance_id: 1,
        },
        100,
        Some(99),
        7_500,
        ClientMessage::Move(protocol::MoveInput {
            client_tick: 333,
            x: 124,
            y: 118,
            direction: 2,
            path: [1, 1, 2, 3, 5, 8, 13, 21],
        }),
    )
}

fn sample_control_packet() -> WirePacket {
    WirePacket::client(
        42,
        RouteKey::LOBBY,
        1,
        None,
        7_000,
        ClientMessage::Hello(ClientHello {
            account_id: 900,
            auth_token: "bench-token".to_string(),
            client_build: "0.1.0-bench".to_string(),
            locale: "en-US".to_string(),
        }),
    )
}

fn bench_datagram(c: &mut Criterion) {
    let codec = WireCodec::default();
    let packet = sample_move_packet();

    c.bench_with_input(
        BenchmarkId::new("encode_datagram", "move"),
        &packet,
        |b, packet| {
            b.iter(|| {
                codec
                    .encode_datagram_frame(QuicChannel::GameplayInput, black_box(packet))
                    .unwrap()
            });
        },
    );

    let bytes = codec
        .encode_datagram_frame(QuicChannel::GameplayInput, &packet)
        .unwrap();
    c.bench_with_input(
        BenchmarkId::new("decode_datagram", "move"),
        &bytes,
        |b, bytes| {
            b.iter(|| codec.decode_datagram_frame(black_box(bytes)).unwrap());
        },
    );
}

fn bench_stream(c: &mut Criterion) {
    let codec = WireCodec::default();
    let packet = sample_control_packet();

    c.bench_with_input(
        BenchmarkId::new("encode_stream", "control"),
        &packet,
        |b, packet| {
            b.iter(|| {
                codec
                    .encode_stream_frame(QuicChannel::Control, black_box(packet))
                    .unwrap()
            });
        },
    );

    let frame = codec
        .encode_stream_frame(QuicChannel::Control, &packet)
        .unwrap();
    c.bench_with_input(
        BenchmarkId::new("decode_stream", "control"),
        &frame,
        |b, frame| {
            b.iter(|| {
                codec
                    .try_decode_stream_frame(black_box(frame))
                    .unwrap()
                    .unwrap()
            });
        },
    );
}

fn protocol_benches(c: &mut Criterion) {
    bench_datagram(c);
    bench_stream(c);
}

criterion_group!(benches, protocol_benches);
criterion_main!(benches);
