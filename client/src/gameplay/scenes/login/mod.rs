//! Login scene: simple 2D-focused state with gray background (no 3D runtime world).

use crate::gameplay::controllers::scene_controller::{SceneController, SceneId};
use crate::gameplay::controllers::world_controller;
use crate::world::{WorldId, WorldRequest};
use bevy::camera::ClearColorConfig;
use bevy::prelude::*;
use bevy::state::prelude::{OnEnter, OnExit};

const LOGIN_BACKGROUND: Color = Color::srgb(0.42, 0.42, 0.42);

pub struct LoginScene;

impl SceneController for LoginScene {
    fn register(app: &mut App) {
        app.add_systems(OnEnter(crate::AppState::Login), setup_login_scene)
            .add_systems(OnExit(crate::AppState::Login), cleanup_login_scene);
    }

    fn scene_id() -> SceneId {
        SceneId::Login
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
    world_controller::request_world(&mut world_requests, WorldId::Loading);

    for mut camera in &mut camera_query {
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

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::state::app::AppExtStates;

    fn count_login_roots(world: &mut bevy::prelude::World) -> usize {
        let mut query = world.query_filtered::<Entity, With<LoginSceneRoot>>();
        query.iter(world).count()
    }

    #[test]
    fn login_scene_contract_and_cleanup() {
        assert_eq!(LoginScene::scene_id(), SceneId::Login);

        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin))
            .init_state::<crate::AppState>()
            .add_message::<WorldRequest>();

        LoginScene::register(&mut app);

        app.update();
        assert_eq!(count_login_roots(app.world_mut()), 1);

        app.world_mut()
            .resource_mut::<NextState<crate::AppState>>()
            .set(crate::AppState::Gameplay);
        app.update();

        assert_eq!(count_login_roots(app.world_mut()), 0);
    }
}
