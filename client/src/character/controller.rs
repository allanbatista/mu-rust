use super::types::CharacterClass;
use bevy::prelude::*;

#[derive(Component)]
pub struct CharacterController {
    pub class: CharacterClass,
    pub state: CharacterState,
}

#[derive(Component)]
pub struct CharacterAnimState {
    pub current_action: usize,
    pub playback_speed: f32,
}

#[derive(Debug, Clone)]
pub enum CharacterState {
    Idle,
    Walking { target: Vec3 },
}
