use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use protocol::header::{PBMSG_HEAD, PSBMSG_HEAD};
use protocol::packets::{CHAT_MESSAGE_LEN, PMSG_CHARACTER_INFO_SEND, PMSG_CHAT_RECV};
use protocol::{de, ser};

fn sample_chat_packet() -> PMSG_CHAT_RECV {
    PMSG_CHAT_RECV {
        header: PBMSG_HEAD {
            r#type: 0xC1,
            size: (core::mem::size_of::<PMSG_CHAT_RECV>()) as u8,
            head: 0x00,
        },
        name: *b"BenchUser\0",
        message: [0x41; CHAT_MESSAGE_LEN],
    }
}

fn sample_character_info_packet() -> PMSG_CHARACTER_INFO_SEND {
    PMSG_CHARACTER_INFO_SEND {
        header: PSBMSG_HEAD {
            r#type: 0xC3,
            size: (core::mem::size_of::<PMSG_CHARACTER_INFO_SEND>()) as u8,
            head: 0xF3,
            subh: 0x03,
        },
        x: 120,
        y: 125,
        map: 3,
        dir: 2,
        experience: [0; 8],
        next_experience: [0; 8],
        level_up_point: 10,
        strength: 200,
        dexterity: 150,
        vitality: 180,
        energy: 140,
        life: 1100,
        max_life: 1200,
        mana: 600,
        max_mana: 650,
        shield: 300,
        max_shield: 320,
        bp: 100,
        max_bp: 120,
        money: 1_000_000,
        pk_level: 3,
        ctl_code: 0,
        fruit_add_point: 0,
        max_fruit_add_point: 0,
        leadership: 0,
        fruit_sub_point: 0,
        max_fruit_sub_point: 0,
        #[cfg(feature = "gameserver_update_ge_602")]
        ext_inventory: 1,
        #[cfg(feature = "gameserver_extra")]
        view_reset: 0,
        #[cfg(feature = "gameserver_extra")]
        view_point: 0,
        #[cfg(feature = "gameserver_extra")]
        view_cur_hp: 1200,
        #[cfg(feature = "gameserver_extra")]
        view_max_hp: 1300,
        #[cfg(feature = "gameserver_extra")]
        view_cur_mp: 600,
        #[cfg(feature = "gameserver_extra")]
        view_max_mp: 650,
        #[cfg(feature = "gameserver_extra")]
        view_cur_bp: 110,
        #[cfg(feature = "gameserver_extra")]
        view_max_bp: 130,
        #[cfg(feature = "gameserver_extra")]
        view_cur_sd: 300,
        #[cfg(feature = "gameserver_extra")]
        view_max_sd: 320,
        #[cfg(feature = "gameserver_extra")]
        view_strength: 200,
        #[cfg(feature = "gameserver_extra")]
        view_dexterity: 150,
        #[cfg(feature = "gameserver_extra")]
        view_vitality: 180,
        #[cfg(feature = "gameserver_extra")]
        view_energy: 140,
        #[cfg(feature = "gameserver_extra")]
        view_leadership: 0,
    }
}

fn bench_chat_packets(c: &mut Criterion) {
    let packet = sample_chat_packet();
    c.bench_with_input(
        BenchmarkId::new("serialize", "chat"),
        &packet,
        |b, packet| {
            b.iter(|| ser::serialize(black_box(packet)));
        },
    );

    let bytes = ser::serialize(&packet);
    c.bench_with_input(
        BenchmarkId::new("deserialize", "chat"),
        &bytes,
        |b, bytes| {
            b.iter(|| de::deserialize::<PMSG_CHAT_RECV>(black_box(bytes)).unwrap());
        },
    );
}

fn bench_character_info_packets(c: &mut Criterion) {
    let packet = sample_character_info_packet();
    c.bench_with_input(
        BenchmarkId::new("serialize", "character_info"),
        &packet,
        |b, packet| {
            b.iter(|| ser::serialize(black_box(packet)));
        },
    );

    let bytes = ser::serialize(&packet);
    c.bench_with_input(
        BenchmarkId::new("deserialize", "character_info"),
        &bytes,
        |b, bytes| {
            b.iter(|| de::deserialize::<PMSG_CHARACTER_INFO_SEND>(black_box(bytes)).unwrap());
        },
    );
}

fn protocol_benches(c: &mut Criterion) {
    bench_chat_packets(c);
    bench_character_info_packets(c);
}

criterion_group!(benches, protocol_benches);
criterion_main!(benches);
