use protocol::de::{DeserializeError, deserialize};
use protocol::header::{PBMSG_HEAD, PSBMSG_HEAD};
use protocol::packets::*;
use protocol::ser::{serialize, serialize_into};

fn example_chat_packet() -> PMSG_CHAT_RECV {
    PMSG_CHAT_RECV {
        header: PBMSG_HEAD {
            r#type: 0xC1,
            size: (core::mem::size_of::<PMSG_CHAT_RECV>()) as u8,
            head: 0x00,
        },
        name: *b"TestPlayer",
        message: {
            let mut buf = [0u8; CHAT_MESSAGE_LEN];
            buf[..13].copy_from_slice(b"Hello World!\0");
            buf
        },
    }
}

fn example_chat_send_packet() -> PMSG_CHAT_SEND {
    PMSG_CHAT_SEND {
        header: PBMSG_HEAD {
            r#type: 0xC1,
            size: (core::mem::size_of::<PMSG_CHAT_SEND>()) as u8,
            head: 0x00,
        },
        name: *b"GameMaster",
        message: {
            let mut buf = [0u8; CHAT_MESSAGE_LEN];
            buf[..16].copy_from_slice(b"Server broadcast");
            buf
        },
    }
}

fn example_action_packet() -> PMSG_ACTION_RECV {
    PMSG_ACTION_RECV {
        header: PBMSG_HEAD {
            r#type: 0xC1,
            size: core::mem::size_of::<PMSG_ACTION_RECV>() as u8,
            head: 0x18,
        },
        dir: 3,
        action: 7,
        index: [0x01, 0x02],
    }
}

fn example_connect_account_packet() -> PMSG_CONNECT_ACCOUNT_RECV {
    PMSG_CONNECT_ACCOUNT_RECV {
        header: PSBMSG_HEAD {
            r#type: 0xC3,
            size: core::mem::size_of::<PMSG_CONNECT_ACCOUNT_RECV>() as u8,
            head: 0xF1,
            subh: 0x01,
        },
        account: {
            let mut buf = [0u8; ACCOUNT_LEN];
            buf[..10].copy_from_slice(b"accnt00001");
            buf
        },
        password: *b"passw0rd1234",
        tick_count: 0xDEADBEEF,
        client_version: [1, 2, 3, 4, 5],
        client_serial: {
            let mut buf = [0u8; CLIENT_SERIAL_LEN];
            buf[..16].copy_from_slice(b"SERIAL-000000000");
            buf
        },
    }
}

#[test]
fn chat_packet_roundtrip() {
    let packet = example_chat_packet();
    let bytes = serialize(&packet);
    let decoded: PMSG_CHAT_RECV = deserialize(&bytes).unwrap();
    assert_eq!(packet, decoded);
}

#[test]
fn chat_send_packet_roundtrip() {
    let packet = example_chat_send_packet();
    let bytes = serialize(&packet);
    let decoded: PMSG_CHAT_SEND = deserialize(&bytes).unwrap();
    assert_eq!(packet, decoded);
}

#[test]
fn action_packet_roundtrip_with_append() {
    let packet = example_action_packet();
    let mut buffer = Vec::new();
    serialize_into(&packet, &mut buffer);
    let decoded: PMSG_ACTION_RECV = deserialize(&buffer).unwrap();
    assert_eq!(packet, decoded);
}

#[test]
fn connect_account_roundtrip() {
    let packet = example_connect_account_packet();
    let bytes = serialize(&packet);
    let decoded: PMSG_CONNECT_ACCOUNT_RECV = deserialize(&bytes).unwrap();
    assert_eq!(packet, decoded);
}

#[test]
fn deserialize_size_mismatch() {
    let bytes = vec![0u8; core::mem::size_of::<PMSG_CHAT_RECV>() - 1];
    let err = deserialize::<PMSG_CHAT_RECV>(&bytes).unwrap_err();
    assert_eq!(
        err,
        DeserializeError::SizeMismatch {
            expected: core::mem::size_of::<PMSG_CHAT_RECV>(),
            actual: bytes.len(),
        }
    );
}

#[test]
fn friend_message_struct_size() {
    assert_eq!(
        core::mem::size_of::<PMSG_FRIEND_MESSAGE_RECV>(),
        4 + 4 + NAME_LEN + SUBJECT_LEN + 1 + 1 + 2 + FRIEND_MESSAGE_TEXT_LEN
    );
}

#[test]
fn serialize_multiple_packets() {
    let packets = [
        serialize(&example_chat_packet()),
        serialize(&example_action_packet()),
        serialize(&example_connect_account_packet()),
    ];
    let total_len: usize = packets.iter().map(|p| p.len()).sum();
    let mut buffer = Vec::with_capacity(total_len);
    for segment in &packets {
        buffer.extend_from_slice(segment);
    }
    assert_eq!(buffer.len(), total_len);
}
