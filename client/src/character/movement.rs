use super::controller::{CharacterAnimState, CharacterController, CharacterState};
use super::types::CharacterRoot;
use bevy::prelude::*;

const WALK_SPEED: f32 = 150.0;
const ARRIVAL_THRESHOLD: f32 = 5.0;
const TURN_SPEED: f32 = 10.0;

/// Advance character position toward movement target each frame.
pub fn advance_character_movement(
    time: Res<Time>,
    mut characters: Query<
        (
            &mut Transform,
            &mut CharacterController,
            &mut CharacterAnimState,
        ),
        With<CharacterRoot>,
    >,
) {
    let dt = time.delta_seconds();

    for (mut transform, mut controller, mut anim_state) in &mut characters {
        let target = match controller.state {
            CharacterState::Walking { target } => target,
            CharacterState::Idle => continue,
        };

        let current = transform.translation;
        let diff = target - current;
        let distance = diff.length();

        if distance < ARRIVAL_THRESHOLD {
            // Arrived — switch to idle
            controller.state = CharacterState::Idle;
            let idle_action = config_helpers::idle_action_for_class(controller.class);
            if anim_state.current_action != idle_action {
                anim_state.current_action = idle_action;
            }
            continue;
        }

        let direction = diff / distance;

        // Rotate to face movement direction (add PI because glTF models face -Z)
        let target_yaw = direction.x.atan2(direction.z) + std::f32::consts::PI;
        let target_rot = Quat::from_rotation_y(target_yaw);
        transform.rotation = transform
            .rotation
            .slerp(target_rot, (TURN_SPEED * dt).min(1.0));

        // Move forward
        let step = (WALK_SPEED * dt).min(distance);
        transform.translation += direction * step;
    }
}

/// Helper module for looking up class defaults without needing the full config resource.
pub(super) mod config_helpers {
    use super::super::types::CharacterClass;

    pub fn idle_action_for_class(class: CharacterClass) -> usize {
        match class {
            CharacterClass::DarkKnight
            | CharacterClass::DarkWizard
            | CharacterClass::MagicGladiator => 1,
            CharacterClass::FairyElf => 2,
            CharacterClass::Summoner => 3,
            CharacterClass::DarkLord => 76,
            CharacterClass::RageFighter => 286,
        }
    }

    pub fn walk_action_for_class(class: CharacterClass) -> usize {
        match class {
            CharacterClass::DarkKnight
            | CharacterClass::DarkWizard
            | CharacterClass::MagicGladiator
            | CharacterClass::DarkLord
            | CharacterClass::RageFighter => 15,
            CharacterClass::FairyElf | CharacterClass::Summoner => 16,
        }
    }
}
