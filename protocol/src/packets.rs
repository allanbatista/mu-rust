//! Packet structure translations from the legacy C++ implementation.

#![allow(non_camel_case_types)]

use crate::header::{PBMSG_HEAD, PSBMSG_HEAD, PSWMSG_HEAD, PWMSG_HEAD};

pub type BYTE = u8;
pub type WORD = u16;
pub type DWORD = u32;

pub const NAME_LEN: usize = 10;
pub const ACCOUNT_LEN: usize = 10;
pub const PASSWORD_SHORT_LEN: usize = 12;
pub const PASSWORD_LONG_LEN: usize = 20;
pub const CHAT_MESSAGE_LEN: usize = 60;
pub const SUBJECT_LEN: usize = 60;
pub const FRIEND_MESSAGE_TEXT_LEN: usize = 1000;
pub const CLIENT_VERSION_LEN: usize = 5;
pub const CLIENT_SERIAL_LEN: usize = 16;

// -----------------------------------------------------------------------------
// Client -> GameServer
// -----------------------------------------------------------------------------

/// Chat message received from the client (`PMSG_CHAT_RECV`).
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PMSG_CHAT_RECV {
    pub header: PBMSG_HEAD,
    pub name: [u8; NAME_LEN],
    pub message: [u8; CHAT_MESSAGE_LEN],
}

/// Whisper chat payload (`PMSG_CHAT_WHISPER_RECV`).
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PMSG_CHAT_WHISPER_RECV {
    pub header: PBMSG_HEAD,
    pub name: [u8; NAME_LEN],
    pub message: [u8; CHAT_MESSAGE_LEN],
}

/// Client heartbeat and status (`PMSG_LIVE_CLIENT_RECV`).
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PMSG_LIVE_CLIENT_RECV {
    pub header: PBMSG_HEAD,
    pub tick_count: DWORD,
    pub physi_speed: WORD,
    pub magic_speed: WORD,
}

/// Player movement packet (`PMSG_MOVE_RECV`).
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PMSG_MOVE_RECV {
    pub header: PBMSG_HEAD,
    pub x: BYTE,
    pub y: BYTE,
    pub path: [BYTE; 8],
}

/// Player position acknowledgement (`PMSG_POSITION_RECV`).
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PMSG_POSITION_RECV {
    pub header: PBMSG_HEAD,
    pub x: BYTE,
    pub y: BYTE,
}

/// Action packet (`PMSG_ACTION_RECV`).
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PMSG_ACTION_RECV {
    pub header: PBMSG_HEAD,
    pub dir: BYTE,
    pub action: BYTE,
    pub index: [BYTE; 2],
}

/// Event remain time notification (`PMSG_EVENT_REMAIN_TIME_RECV`).
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PMSG_EVENT_REMAIN_TIME_RECV {
    pub header: PBMSG_HEAD,
    pub event_type: BYTE,
    pub item_level: BYTE,
}

/// Pet item command request (`PMSG_PET_ITEM_COMMAND_RECV`).
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PMSG_PET_ITEM_COMMAND_RECV {
    pub header: PBMSG_HEAD,
    pub r#type: BYTE,
    pub command: BYTE,
    pub index: [BYTE; 2],
}

/// Pet item info request (`PMSG_PET_ITEM_INFO_RECV`).
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PMSG_PET_ITEM_INFO_RECV {
    pub header: PBMSG_HEAD,
    pub r#type: BYTE,
    pub flag: BYTE,
    pub slot: BYTE,
}

/// Map server move auth packet (`PMSG_MAP_SERVER_MOVE_AUTH_RECV`).
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PMSG_MAP_SERVER_MOVE_AUTH_RECV {
    pub header: PSBMSG_HEAD,
    pub account: [u8; 12],
    pub name: [u8; 12],
    pub auth_code1: DWORD,
    pub auth_code2: DWORD,
    pub auth_code3: DWORD,
    pub auth_code4: DWORD,
    pub tick_count: DWORD,
    pub client_version: [BYTE; CLIENT_VERSION_LEN],
    pub client_serial: [BYTE; CLIENT_SERIAL_LEN],
}

/// Friend message payload (`PMSG_FRIEND_MESSAGE_RECV`).
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PMSG_FRIEND_MESSAGE_RECV {
    pub header: PWMSG_HEAD,
    pub guid: DWORD,
    pub name: [u8; NAME_LEN],
    pub subject: [u8; SUBJECT_LEN],
    pub dir: BYTE,
    pub action: BYTE,
    pub size: WORD,
    pub text: [u8; FRIEND_MESSAGE_TEXT_LEN],
}

/// Account connection request (`PMSG_CONNECT_ACCOUNT_RECV`).
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PMSG_CONNECT_ACCOUNT_RECV {
    pub header: PSBMSG_HEAD,
    pub account: [u8; ACCOUNT_LEN],
    pub password: [u8; PASSWORD_SHORT_LEN],
    pub tick_count: DWORD,
    pub client_version: [BYTE; CLIENT_VERSION_LEN],
    pub client_serial: [BYTE; CLIENT_SERIAL_LEN],
}

/// Client disconnect notice (`PMSG_CLOSE_CLIENT_RECV`).
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PMSG_CLOSE_CLIENT_RECV {
    pub header: PSBMSG_HEAD,
    pub r#type: BYTE,
}

/// Character creation (`PMSG_CHARACTER_CREATE_RECV`).
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PMSG_CHARACTER_CREATE_RECV {
    pub header: PSBMSG_HEAD,
    pub name: [u8; NAME_LEN],
    pub class: BYTE,
}

/// Character delete (`PMSG_CHARACTER_DELETE_RECV`).
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PMSG_CHARACTER_DELETE_RECV {
    pub header: PSBMSG_HEAD,
    pub name: [u8; NAME_LEN],
    pub personal_code: [u8; 10],
}

/// Character info request (`PMSG_CHARACTER_INFO_RECV`).
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PMSG_CHARACTER_INFO_RECV {
    pub header: PSBMSG_HEAD,
    pub name: [u8; NAME_LEN],
}

/// Level up point request (`PMSG_LEVEL_UP_POINT_RECV`).
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PMSG_LEVEL_UP_POINT_RECV {
    pub header: PSBMSG_HEAD,
    pub r#type: BYTE,
}

/// Character name check (`PMSG_CHARACTER_NAME_CHECK_RECV`).
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PMSG_CHARACTER_NAME_CHECK_RECV {
    pub header: PSBMSG_HEAD,
    pub name: [u8; NAME_LEN],
}

/// Character name change (`PMSG_CHARACTER_NAME_CHANGE_RECV`).
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PMSG_CHARACTER_NAME_CHANGE_RECV {
    pub header: PSBMSG_HEAD,
    pub old_name: [u8; NAME_LEN],
    pub new_name: [u8; NAME_LEN],
}

/// Option change skin (`PMSG_OPTION_CHANGE_SKIN_RECV`).
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PMSG_OPTION_CHANGE_SKIN_RECV {
    pub header: PSBMSG_HEAD,
    pub change_skin: BYTE,
}

/// Option data update (`PMSG_OPTION_DATA_RECV`).
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PMSG_OPTION_DATA_RECV {
    pub header: PSBMSG_HEAD,
    pub skill_key: [BYTE; 20],
    pub game_option: BYTE,
    pub q_key: BYTE,
    pub w_key: BYTE,
    pub e_key: BYTE,
    pub chat_window: BYTE,
    pub r_key: BYTE,
    pub qwer_level: DWORD,
}

/// Client security breach notification (`PMSG_CLIENT_SECURITY_BREACH_RECV`).
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PMSG_CLIENT_SECURITY_BREACH_RECV {
    pub header: PSBMSG_HEAD,
    pub code: [BYTE; 4],
}

/// SNS data request (`PMSG_SNS_DATA_RECV`).
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PMSG_SNS_DATA_RECV {
    pub header: PWMSG_HEAD,
    pub result: BYTE,
    pub data: [BYTE; 256],
}

/// SNS data log request (`PMSG_SNS_DATA_LOG_RECV`).
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PMSG_SNS_DATA_LOG_RECV {
    pub header: PBMSG_HEAD,
    pub code: [BYTE; 3],
}

/// Off-trade request (`PMSG_OFFTRADE_RECV`).
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PMSG_OFFTRADE_RECV {
    pub header: PSBMSG_HEAD,
    pub r#type: i32,
}

// -----------------------------------------------------------------------------
// GameServer -> Client
// -----------------------------------------------------------------------------

/// Broadcast chat from server (`PMSG_CHAT_SEND`).
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PMSG_CHAT_SEND {
    pub header: PBMSG_HEAD,
    pub name: [u8; NAME_LEN],
    pub message: [u8; CHAT_MESSAGE_LEN],
}

/// Directed chat to specific player (`PMSG_CHAT_TARGET_SEND`).
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PMSG_CHAT_TARGET_SEND {
    pub header: PBMSG_HEAD,
    pub index: [BYTE; 2],
    pub message: [u8; CHAT_MESSAGE_LEN],
}

/// Whisper chat from server (`PMSG_CHAT_WHISPER_SEND`).
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PMSG_CHAT_WHISPER_SEND {
    pub header: PBMSG_HEAD,
    pub name: [u8; NAME_LEN],
    pub message: [u8; CHAT_MESSAGE_LEN],
}

/// Main check response (`PMSG_MAIN_CHECK_SEND`).
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PMSG_MAIN_CHECK_SEND {
    pub header: PBMSG_HEAD,
    pub key: WORD,
}

/// Event state update (`PMSG_EVENT_STATE_SEND`).
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PMSG_EVENT_STATE_SEND {
    pub header: PBMSG_HEAD,
    pub state: BYTE,
    pub event: BYTE,
}

/// Server message index (`PMSG_SERVER_MSG_SEND`).
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PMSG_SERVER_MSG_SEND {
    pub header: PBMSG_HEAD,
    pub msg_number: BYTE,
}

/// Weather change notification (`PMSG_WEATHER_SEND`).
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PMSG_WEATHER_SEND {
    pub header: PBMSG_HEAD,
    pub weather: BYTE,
}

/// Monster death notification (`PMSG_MONSTER_DIE_SEND`).
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PMSG_MONSTER_DIE_SEND {
    pub header: PBMSG_HEAD,
    pub index: [BYTE; 2],
    pub experience: [BYTE; 2],
    pub damage: [BYTE; 2],
    #[cfg(feature = "gameserver_extra")]
    pub view_damage_hp: DWORD,
}

/// User death notification (`PMSG_USER_DIE_SEND`).
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PMSG_USER_DIE_SEND {
    pub header: PBMSG_HEAD,
    pub index: [BYTE; 2],
    pub skill: [BYTE; 2],
    pub killer: [BYTE; 2],
}

/// Action broadcast (`PMSG_ACTION_SEND`).
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PMSG_ACTION_SEND {
    pub header: PBMSG_HEAD,
    pub index: [BYTE; 2],
    pub dir: BYTE,
    pub action: BYTE,
    pub target: [BYTE; 2],
}

/// Life update (`PMSG_LIFE_SEND`).
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PMSG_LIFE_SEND {
    pub header: PBMSG_HEAD,
    pub r#type: BYTE,
    pub life: [BYTE; 2],
    pub flag: BYTE,
    pub shield: [BYTE; 2],
    #[cfg(feature = "gameserver_extra")]
    pub view_hp: DWORD,
    #[cfg(feature = "gameserver_extra")]
    pub view_sd: DWORD,
}

/// Move broadcast (`PMSG_MOVE_SEND`).
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PMSG_MOVE_SEND {
    pub header: PBMSG_HEAD,
    pub index: [BYTE; 2],
    pub x: BYTE,
    pub y: BYTE,
    pub dir: BYTE,
}

/// Elemental damage notification (`PMSG_ELEMENTAL_DAMAGE_SEND`).
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PMSG_ELEMENTAL_DAMAGE_SEND {
    pub header: PBMSG_HEAD,
    pub index: [BYTE; 2],
    pub attribute: BYTE,
    pub damage: DWORD,
    #[cfg(feature = "gameserver_extra")]
    pub view_cur_hp: DWORD,
    #[cfg(feature = "gameserver_extra")]
    pub view_cur_sd: DWORD,
}

/// Character creation enable (`PMSG_CHARACTER_CREATION_ENABLE_SEND`).
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PMSG_CHARACTER_CREATION_ENABLE_SEND {
    pub header: PBMSG_HEAD,
    pub flag: BYTE,
    pub result: BYTE,
}

/// Life update with 32-bit fields (`PMSG_LIFE_UPDATE_SEND`).
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PMSG_LIFE_UPDATE_SEND {
    pub header: PSBMSG_HEAD,
    pub index: [BYTE; 2],
    pub max_hp: [BYTE; 4],
    pub cur_hp: [BYTE; 4],
}

/// Attack speed update (`PMSG_CHARACTER_ATTACK_SPEED_SEND`).
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PMSG_CHARACTER_ATTACK_SPEED_SEND {
    pub header: PSBMSG_HEAD,
    pub physi_speed: DWORD,
    pub magic_speed: DWORD,
}

/// Event map error (`PMSG_ENTER_EVENT_MAP_ERROR_SEND`).
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PMSG_ENTER_EVENT_MAP_ERROR_SEND {
    pub header: PSBMSG_HEAD,
    pub result: DWORD,
}

/// Connection acknowledgement variant (`PMSG_CONNECT_CLIENT_SEND2`).
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PMSG_CONNECT_CLIENT_SEND2 {
    pub header: PSBMSG_HEAD,
    pub result: BYTE,
    pub index: [BYTE; 2],
    pub client_version: [BYTE; CLIENT_VERSION_LEN],
}

/// Connection acknowledgement (`PMSG_CONNECT_CLIENT_SEND`).
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PMSG_CONNECT_CLIENT_SEND {
    pub header: PSBMSG_HEAD,
    pub result: BYTE,
    pub index: [BYTE; 2],
    pub client_version: [BYTE; CLIENT_VERSION_LEN],
}

/// Account connection response (`PMSG_CONNECT_ACCOUNT_SEND`).
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PMSG_CONNECT_ACCOUNT_SEND {
    pub header: PSBMSG_HEAD,
    pub result: BYTE,
}

/// Client close response (`PMSG_CLOSE_CLIENT_SEND`).
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PMSG_CLOSE_CLIENT_SEND {
    pub header: PSBMSG_HEAD,
    pub result: BYTE,
}

/// Character list header (`PMSG_CHARACTER_LIST_SEND`).
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PMSG_CHARACTER_LIST_SEND {
    pub header: PSBMSG_HEAD,
    pub class_code: BYTE,
    pub move_cnt: BYTE,
    pub count: BYTE,
}

/// Character list entry.
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PMSG_CHARACTER_LIST {
    pub slot: BYTE,
    pub name: [u8; NAME_LEN],
    pub level: WORD,
    pub ctl_code: BYTE,
    pub charset: [BYTE; 18],
    pub guild_status: BYTE,
}

/// Character create response (`PMSG_CHARACTER_CREATE_SEND`).
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PMSG_CHARACTER_CREATE_SEND {
    pub header: PSBMSG_HEAD,
    pub result: BYTE,
    pub name: [u8; NAME_LEN],
    pub slot: BYTE,
    pub level: WORD,
    pub class: BYTE,
    pub equipment: [BYTE; 24],
}

/// Character delete response (`PMSG_CHARACTER_DELETE_SEND`).
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PMSG_CHARACTER_DELETE_SEND {
    pub header: PSBMSG_HEAD,
    pub result: BYTE,
}

/// Character info payload (`PMSG_CHARACTER_INFO_SEND`).
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PMSG_CHARACTER_INFO_SEND {
    pub header: PSBMSG_HEAD,
    pub x: BYTE,
    pub y: BYTE,
    pub map: BYTE,
    pub dir: BYTE,
    pub experience: [BYTE; 8],
    pub next_experience: [BYTE; 8],
    pub level_up_point: WORD,
    pub strength: WORD,
    pub dexterity: WORD,
    pub vitality: WORD,
    pub energy: WORD,
    pub life: WORD,
    pub max_life: WORD,
    pub mana: WORD,
    pub max_mana: WORD,
    pub shield: WORD,
    pub max_shield: WORD,
    pub bp: WORD,
    pub max_bp: WORD,
    pub money: DWORD,
    pub pk_level: BYTE,
    pub ctl_code: BYTE,
    pub fruit_add_point: WORD,
    pub max_fruit_add_point: WORD,
    pub leadership: WORD,
    pub fruit_sub_point: WORD,
    pub max_fruit_sub_point: WORD,
    #[cfg(feature = "gameserver_update_ge_602")]
    pub ext_inventory: BYTE,
    #[cfg(feature = "gameserver_extra")]
    pub view_reset: DWORD,
    #[cfg(feature = "gameserver_extra")]
    pub view_point: DWORD,
    #[cfg(feature = "gameserver_extra")]
    pub view_cur_hp: DWORD,
    #[cfg(feature = "gameserver_extra")]
    pub view_max_hp: DWORD,
    #[cfg(feature = "gameserver_extra")]
    pub view_cur_mp: DWORD,
    #[cfg(feature = "gameserver_extra")]
    pub view_max_mp: DWORD,
    #[cfg(feature = "gameserver_extra")]
    pub view_cur_bp: DWORD,
    #[cfg(feature = "gameserver_extra")]
    pub view_max_bp: DWORD,
    #[cfg(feature = "gameserver_extra")]
    pub view_cur_sd: DWORD,
    #[cfg(feature = "gameserver_extra")]
    pub view_max_sd: DWORD,
    #[cfg(feature = "gameserver_extra")]
    pub view_strength: DWORD,
    #[cfg(feature = "gameserver_extra")]
    pub view_dexterity: DWORD,
    #[cfg(feature = "gameserver_extra")]
    pub view_vitality: DWORD,
    #[cfg(feature = "gameserver_extra")]
    pub view_energy: DWORD,
    #[cfg(feature = "gameserver_extra")]
    pub view_leadership: DWORD,
}

/// Character regeneration payload (`PMSG_CHARACTER_REGEN_SEND`).
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PMSG_CHARACTER_REGEN_SEND {
    pub header: PSBMSG_HEAD,
    pub x: BYTE,
    pub y: BYTE,
    pub map: BYTE,
    pub dir: BYTE,
    pub life: WORD,
    pub mana: WORD,
    pub shield: WORD,
    pub bp: WORD,
    pub experience: [BYTE; 8],
    pub money: DWORD,
    #[cfg(feature = "gameserver_extra")]
    pub view_cur_hp: DWORD,
    #[cfg(feature = "gameserver_extra")]
    pub view_cur_mp: DWORD,
    #[cfg(feature = "gameserver_extra")]
    pub view_cur_bp: DWORD,
    #[cfg(feature = "gameserver_extra")]
    pub view_cur_sd: DWORD,
}

/// Level up payload (`PMSG_LEVEL_UP_SEND`).
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PMSG_LEVEL_UP_SEND {
    pub header: PSBMSG_HEAD,
    pub level: WORD,
    pub level_up_point: WORD,
    pub max_life: WORD,
    pub max_mana: WORD,
    pub max_shield: WORD,
    pub max_bp: WORD,
    pub fruit_add_point: WORD,
    pub max_fruit_add_point: WORD,
    pub fruit_sub_point: WORD,
    pub max_fruit_sub_point: WORD,
    #[cfg(feature = "gameserver_extra")]
    pub view_point: DWORD,
    #[cfg(feature = "gameserver_extra")]
    pub view_max_hp: DWORD,
    #[cfg(feature = "gameserver_extra")]
    pub view_max_mp: DWORD,
    #[cfg(feature = "gameserver_extra")]
    pub view_max_bp: DWORD,
    #[cfg(feature = "gameserver_extra")]
    pub view_max_sd: DWORD,
    #[cfg(feature = "gameserver_extra")]
    pub view_experience: DWORD,
    #[cfg(feature = "gameserver_extra")]
    pub view_next_experience: DWORD,
}

/// Level up point change (`PMSG_LEVEL_UP_POINT_SEND`).
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PMSG_LEVEL_UP_POINT_SEND {
    pub header: PSBMSG_HEAD,
    pub result: BYTE,
    pub max_life_and_mana: WORD,
    pub max_shield: WORD,
    pub max_bp: WORD,
    #[cfg(feature = "gameserver_extra")]
    pub view_point: DWORD,
    #[cfg(feature = "gameserver_extra")]
    pub view_max_hp: DWORD,
    #[cfg(feature = "gameserver_extra")]
    pub view_max_mp: DWORD,
    #[cfg(feature = "gameserver_extra")]
    pub view_max_bp: DWORD,
    #[cfg(feature = "gameserver_extra")]
    pub view_max_sd: DWORD,
}

/// Extended character calculation payload (`PMSG_NEW_CHARACTER_CALC_SEND`).
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PMSG_NEW_CHARACTER_CALC_SEND {
    pub header: PSBMSG_HEAD,
    pub view_cur_hp: DWORD,
    pub view_max_hp: DWORD,
    pub view_cur_mp: DWORD,
    pub view_max_mp: DWORD,
    pub view_cur_bp: DWORD,
    pub view_max_bp: DWORD,
    pub view_cur_sd: DWORD,
    pub view_max_sd: DWORD,
    pub view_add_strength: DWORD,
    pub view_add_dexterity: DWORD,
    pub view_add_vitality: DWORD,
    pub view_add_energy: DWORD,
    pub view_add_leadership: DWORD,
    pub view_physi_damage_min: DWORD,
    pub view_physi_damage_max: DWORD,
    pub view_magic_damage_min: DWORD,
    pub view_magic_damage_max: DWORD,
    pub view_curse_damage_min: DWORD,
    pub view_curse_damage_max: DWORD,
    pub view_mul_physi_damage: DWORD,
    pub view_div_physi_damage: DWORD,
    pub view_mul_magic_damage: DWORD,
    pub view_div_magic_damage: DWORD,
    pub view_mul_curse_damage: DWORD,
    pub view_div_curse_damage: DWORD,
    pub view_magic_damage_rate: DWORD,
    pub view_curse_damage_rate: DWORD,
    pub view_physi_speed: DWORD,
    pub view_magic_speed: DWORD,
    pub view_attack_success_rate: DWORD,
    pub view_attack_success_rate_pvp: DWORD,
    pub view_defense: DWORD,
    pub view_defense_success_rate: DWORD,
    pub view_defense_success_rate_pvp: DWORD,
    pub view_damage_multiplier: DWORD,
    pub view_rf_damage_multiplier_a: DWORD,
    pub view_rf_damage_multiplier_b: DWORD,
    pub view_rf_damage_multiplier_c: DWORD,
    pub view_dark_spirit_attack_damage_min: DWORD,
    pub view_dark_spirit_attack_damage_max: DWORD,
    pub view_dark_spirit_attack_speed: DWORD,
    pub view_dark_spirit_attack_success_rate: DWORD,
}

/// Health bar collection (`PMSG_NEW_HEALTH_BAR_SEND`).
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PMSG_NEW_HEALTH_BAR_SEND {
    pub header: PSWMSG_HEAD,
    pub count: BYTE,
}

/// Health bar entry (`PMSG_NEW_HEALTH_BAR`).
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PMSG_NEW_HEALTH_BAR {
    pub index: WORD,
    pub r#type: BYTE,
    pub rate: BYTE,
    pub rate2: BYTE,
}

/// Gens battle info (`PMSG_NEW_GENS_BATTLE_INFO_SEND`).
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PMSG_NEW_GENS_BATTLE_INFO_SEND {
    pub header: PSBMSG_HEAD,
    pub gens_battle_map_count: BYTE,
    pub gens_move_index_count: BYTE,
    pub gens_battle_map: [BYTE; 120],
    pub gens_move_index: [BYTE; 120],
}

/// Server message with inline string (`PMSG_NEW_MESSAGE_SEND`).
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PMSG_NEW_MESSAGE_SEND {
    pub header: PSBMSG_HEAD,
    pub message: [u8; 128],
}

/// Off-trade response (`PMSG_OFFTRADE_SEND`).
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PMSG_OFFTRADE_SEND {
    pub header: PSBMSG_HEAD,
    pub r#type: i32,
}

/// Shop active response (`PMSG_SHOPACTIVE_SEND`).
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PMSG_SHOPACTIVE_SEND {
    pub header: PSBMSG_HEAD,
    pub active: i32,
    pub r#type: i32,
}

/// Ping keep-alive (`PMSG_PING_SEND`).
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PMSG_PING_SEND {
    pub header: PSBMSG_HEAD,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn size_matches_cpp_layouts_basic() {
        assert_eq!(
            core::mem::size_of::<PMSG_CHAT_RECV>(),
            3 + NAME_LEN + CHAT_MESSAGE_LEN
        );
        assert_eq!(
            core::mem::size_of::<PMSG_CHAT_WHISPER_RECV>(),
            3 + NAME_LEN + CHAT_MESSAGE_LEN
        );
        assert_eq!(core::mem::size_of::<PMSG_LIVE_CLIENT_RECV>(), 3 + 4 + 2 + 2);
        assert_eq!(core::mem::size_of::<PMSG_MOVE_RECV>(), 3 + 1 + 1 + 8);
        assert_eq!(core::mem::size_of::<PMSG_POSITION_RECV>(), 3 + 1 + 1);
        assert_eq!(core::mem::size_of::<PMSG_ACTION_RECV>(), 3 + 1 + 1 + 2);
        assert_eq!(
            core::mem::size_of::<PMSG_EVENT_REMAIN_TIME_RECV>(),
            3 + 1 + 1
        );
        assert_eq!(
            core::mem::size_of::<PMSG_PET_ITEM_COMMAND_RECV>(),
            3 + 1 + 1 + 2
        );
        assert_eq!(
            core::mem::size_of::<PMSG_PET_ITEM_INFO_RECV>(),
            3 + 1 + 1 + 1
        );
        assert_eq!(
            core::mem::size_of::<PMSG_MAP_SERVER_MOVE_AUTH_RECV>(),
            4 + 12 + 12 + (4 * 5) + CLIENT_VERSION_LEN + CLIENT_SERIAL_LEN
        );
        assert_eq!(
            core::mem::size_of::<PMSG_FRIEND_MESSAGE_RECV>(),
            4 + 4 + NAME_LEN + SUBJECT_LEN + 1 + 1 + 2 + FRIEND_MESSAGE_TEXT_LEN
        );
        assert_eq!(
            core::mem::size_of::<PMSG_CONNECT_ACCOUNT_RECV>(),
            4 + ACCOUNT_LEN + PASSWORD_SHORT_LEN + 4 + CLIENT_VERSION_LEN + CLIENT_SERIAL_LEN
        );
        assert_eq!(core::mem::size_of::<PMSG_CLOSE_CLIENT_RECV>(), 4 + 1);
        assert_eq!(
            core::mem::size_of::<PMSG_CHAT_SEND>(),
            3 + NAME_LEN + CHAT_MESSAGE_LEN
        );
        assert_eq!(
            core::mem::size_of::<PMSG_CHAT_TARGET_SEND>(),
            3 + 2 + CHAT_MESSAGE_LEN
        );
        assert_eq!(core::mem::size_of::<PMSG_MAIN_CHECK_SEND>(), 3 + 2);
        assert_eq!(core::mem::size_of::<PMSG_EVENT_STATE_SEND>(), 3 + 1 + 1);
        assert_eq!(core::mem::size_of::<PMSG_SERVER_MSG_SEND>(), 3 + 1);
        assert_eq!(core::mem::size_of::<PMSG_WEATHER_SEND>(), 3 + 1);
        assert_eq!(core::mem::size_of::<PMSG_USER_DIE_SEND>(), 3 + 2 + 2 + 2);
        assert_eq!(core::mem::size_of::<PMSG_ACTION_SEND>(), 3 + 2 + 1 + 1 + 2);
        assert_eq!(core::mem::size_of::<PMSG_MOVE_SEND>(), 3 + 2 + 1 + 1 + 1);
        assert_eq!(
            core::mem::size_of::<PMSG_CHARACTER_LIST>(),
            1 + NAME_LEN + 2 + 1 + 18 + 1
        );
    }

    #[test]
    fn size_matches_character_packets() {
        assert_eq!(
            core::mem::size_of::<PMSG_CHARACTER_CREATE_SEND>(),
            4 + 1 + NAME_LEN + 1 + 2 + 1 + 24
        );
        assert_eq!(core::mem::size_of::<PMSG_CHARACTER_DELETE_SEND>(), 4 + 1);

        let mut expected_info = 4 // header
            + 4 // coordinates/map/dir
            + 8 // experience
            + 8 // next experience
            + (18 * 2) // WORD stats
            + 4 // money
            + 1 // pk level
            + 1; // ctl code
        if cfg!(feature = "gameserver_update_ge_602") {
            expected_info += 1;
        }
        if cfg!(feature = "gameserver_extra") {
            expected_info += 15 * 4;
        }
        assert_eq!(
            core::mem::size_of::<PMSG_CHARACTER_INFO_SEND>(),
            expected_info
        );
    }
}
