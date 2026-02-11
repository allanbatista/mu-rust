//! Login scene: simple 2D-focused state with gray background (no 3D runtime world).

use crate::world::{WorldId, WorldRequest};
use bevy::camera::ClearColorConfig;
use bevy::prelude::*;
use bevy::state::prelude::{OnEnter, OnExit};

use super::SceneBuilder;

const LOGIN_BACKGROUND: Color = Color::srgb(0.42, 0.42, 0.42);

pub struct LoginScene;

impl SceneBuilder for LoginScene {
    fn register(app: &mut App) {
        app.add_systems(OnEnter(crate::AppState::Login), setup_login_scene)
            .add_systems(OnExit(crate::AppState::Login), cleanup_login_scene);
    }
}

#[derive(Component)]
struct LoginSceneRoot;

fn setup_login_scene(
    mut commands: Commands,
    mut world_requests: MessageWriter<WorldRequest>,
    mut camera_query: Query<&mut Camera, With<Camera3d>>,
) {
    info!("Setting up login scene (simple gray background)");

    // Keep login lightweight: no world/runtime assets.
    world_requests.write(WorldRequest(WorldId::Loading));

    if let Ok(mut camera) = camera_query.single_mut() {
        camera.clear_color = ClearColorConfig::Custom(LOGIN_BACKGROUND);
    }

    commands.spawn((LoginSceneRoot, Transform::default(), Visibility::Visible));
}

fn cleanup_login_scene(mut commands: Commands, query: Query<Entity, With<LoginSceneRoot>>) {
    info!("Cleaning up login scene");

    for entity in &query {
        commands.entity(entity).try_despawn();
    }
}
