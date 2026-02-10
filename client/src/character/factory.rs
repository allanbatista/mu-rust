use super::controller::{CharacterAnimState, CharacterController, CharacterState};
use super::types::{BodyPartMarker, BodySlot, CharacterClass, CharacterRoot};
use bevy::prelude::*;

pub struct CharacterFactory;

impl CharacterFactory {
    pub fn spawn(
        commands: &mut Commands,
        asset_server: &AssetServer,
        class: CharacterClass,
        position: Vec3,
        idle_action: usize,
        playback_speed: f32,
    ) -> Entity {
        let body_type = class.body_type();
        let slots = BodySlot::slots_for(body_type);

        let root = commands
            .spawn((
                SpatialBundle {
                    transform: Transform::from_translation(position),
                    ..default()
                },
                CharacterRoot,
                CharacterController {
                    class,
                    state: CharacterState::Idle,
                },
                CharacterAnimState {
                    current_action: idle_action,
                    playback_speed,
                },
            ))
            .id();

        for &slot in slots {
            let glb_path = slot.default_glb_path(body_type);
            let scene_path = format!("{glb_path}#Scene0");
            let scene_handle: Handle<Scene> = asset_server.load(scene_path);

            let part = commands
                .spawn((
                    SceneBundle {
                        scene: scene_handle,
                        ..default()
                    },
                    BodyPartMarker { slot },
                ))
                .id();

            commands.entity(root).add_child(part);
        }

        root
    }
}
