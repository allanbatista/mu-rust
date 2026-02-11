use super::controller::{CharacterAnimState, CharacterController, CharacterState};
use super::types::CharacterRoot;
use bevy::prelude::*;

const WALK_SPEED: f32 = 300.0;
const RUN_SPEED: f32 = 375.0;
const ARRIVAL_THRESHOLD: f32 = 5.0;
const TURN_SPEED: f32 = 10.0;
const RUN_TO_WALK_THRESHOLD: f32 = 300.0;
const MODEL_YAW_OFFSET: f32 = 0.0;

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
    let dt = time.delta_secs();

    for (mut transform, mut controller, mut anim_state) in &mut characters {
        let (target, speed, movement_action, movement_speed) = match controller.state {
            CharacterState::Walking { target } => (
                target,
                WALK_SPEED,
                config_helpers::walk_action_for_class(controller.class),
                config_helpers::walk_playback_speed(controller.class),
            ),
            CharacterState::Running { target } => (
                target,
                RUN_SPEED,
                config_helpers::run_action_for_class(controller.class),
                config_helpers::run_playback_speed(controller.class),
            ),
            CharacterState::Idle => continue,
        };

        if anim_state.current_action != movement_action {
            anim_state.current_action = movement_action;
        }
        if (anim_state.playback_speed - movement_speed).abs() > f32::EPSILON {
            anim_state.playback_speed = movement_speed;
        }

        let diff = Vec3::new(
            target.x - transform.translation.x,
            0.0,
            target.z - transform.translation.z,
        );
        let distance = diff.length();

        if distance < ARRIVAL_THRESHOLD {
            transform.translation = target;
            controller.state = CharacterState::Idle;
            let idle_action = config_helpers::idle_action_for_class(controller.class);
            if anim_state.current_action != idle_action {
                anim_state.current_action = idle_action;
            }
            anim_state.playback_speed = config_helpers::idle_playback_speed(controller.class);
            continue;
        }

        // Match legacy behavior: run first, then walk on final approach.
        if matches!(controller.state, CharacterState::Running { .. })
            && distance < RUN_TO_WALK_THRESHOLD
        {
            controller.state = CharacterState::Walking { target };
            continue;
        }

        let direction = diff / distance;

        // SourceMain uses CreateAngle(dx, dy) for heading. After MU->glTF basis conversion,
        // the equivalent Bevy yaw is the negated MU heading.
        let target_yaw = mu_heading_to_bevy_yaw(direction.x, direction.z) + MODEL_YAW_OFFSET;
        let target_rot = Quat::from_rotation_y(target_yaw);
        transform.rotation = transform
            .rotation
            .slerp(target_rot, (TURN_SPEED * dt).min(1.0));

        let step = (speed * dt).min(distance);
        transform.translation.x += direction.x * step;
        transform.translation.z += direction.z * step;

        // Keep vertical motion stable and converging to target height.
        let progress = if distance > f32::EPSILON {
            step / distance
        } else {
            1.0
        };
        transform.translation.y += (target.y - transform.translation.y) * progress;
    }
}

fn mu_heading_to_bevy_yaw(direction_x: f32, direction_z: f32) -> f32 {
    let mu_heading = direction_x.atan2(-direction_z);
    -mu_heading
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

    pub fn run_action_for_class(_class: CharacterClass) -> usize {
        // C++/muonline-cross base run action when no weapon-specific override applies.
        25
    }

    pub fn idle_playback_speed(_class: CharacterClass) -> f32 {
        0.16
    }

    pub fn walk_playback_speed(class: CharacterClass) -> f32 {
        match class {
            CharacterClass::RageFighter => 0.32,
            _ => 0.33,
        }
    }

    pub fn run_playback_speed(class: CharacterClass) -> f32 {
        match class {
            CharacterClass::RageFighter => 0.28,
            _ => 0.34,
        }
    }
}
