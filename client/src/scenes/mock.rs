use super::SceneBuilder;
use bevy::prelude::*;
use bevy::state::prelude::{OnEnter, OnExit};

#[derive(Component)]
struct MockSceneRoot;

pub struct MockScene;

impl SceneBuilder for MockScene {
    fn register(app: &mut App) {
        app.add_systems(OnEnter(crate::AppState::Mock), enter_mock_scene)
            .add_systems(OnExit(crate::AppState::Mock), exit_mock_scene);
    }
}

fn enter_mock_scene(mut commands: Commands) {
    commands.spawn((
        MockSceneRoot,
        Sprite {
            color: Color::srgb(0.2, 0.6, 0.3),
            custom_size: Some(Vec2::new(400.0, 200.0)),
            ..Default::default()
        },
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));
}

fn exit_mock_scene(mut commands: Commands, query: Query<Entity, With<MockSceneRoot>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}
