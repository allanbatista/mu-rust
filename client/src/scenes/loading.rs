use super::SceneBuilder;
use crate::world::{WorldId, WorldRequest};
use bevy::prelude::*;
use bevy::sprite::Anchor;
use bevy::state::prelude::{NextState, OnEnter, OnExit, in_state};
use bevy::window::PrimaryWindow;

const LOADING_DURATION_SECS: f32 = 2.0;

#[derive(Component)]
struct LoadingSceneRoot;

#[derive(Component)]
struct LoadingImage {
    handle: Handle<Image>,
}

#[derive(Resource)]
struct LoadingTimer(Timer);

pub struct LoadingScene;

impl SceneBuilder for LoadingScene {
    fn register(app: &mut App) {
        app.add_systems(
            OnEnter(crate::AppState::Loading),
            (setup_loading_scene, start_loading_timer),
        )
        .add_systems(
            Update,
            fit_loading_image.run_if(in_state(crate::AppState::Loading)),
        )
        .add_systems(
            Update,
            advance_loading_timer.run_if(in_state(crate::AppState::Loading)),
        )
        .add_systems(OnExit(crate::AppState::Loading), cleanup_loading_scene);
    }
}

fn setup_loading_scene(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut world_requests: MessageWriter<WorldRequest>,
) {
    world_requests.write(WorldRequest(WorldId::Loading));

    let texture = asset_server.load("wallpapers/entry.png");

    commands
        .spawn((LoadingSceneRoot, Transform::default()))
        .with_children(|parent| {
            parent.spawn((
                LoadingImage {
                    handle: texture.clone(),
                },
                Sprite {
                    image: texture,
                    custom_size: None,
                    ..Default::default()
                },
                Anchor::CENTER,
            ));
        });
}

fn start_loading_timer(mut commands: Commands) {
    commands.insert_resource(LoadingTimer(Timer::from_seconds(
        LOADING_DURATION_SECS,
        TimerMode::Once,
    )));
}

fn advance_loading_timer(
    time: Res<Time>,
    mut timer: ResMut<LoadingTimer>,
    mut next_state: ResMut<NextState<crate::AppState>>,
) {
    if timer.0.tick(time.delta()).just_finished() {
        next_state.set(crate::AppState::Mock);
    }
}

fn fit_loading_image(
    windows: Query<&Window, With<PrimaryWindow>>,
    images: Res<Assets<Image>>,
    mut query: Query<(&LoadingImage, &mut Sprite, &mut Transform)>,
) {
    let Ok(window) = windows.single() else {
        return;
    };

    let Ok((loading_image, mut sprite, mut transform)) = query.single_mut() else {
        return;
    };

    if let Some(image) = images.get(&loading_image.handle) {
        let size = image.size_f32();
        let width = size.x;
        let height = size.y;
        if width > 0.0 && height > 0.0 {
            let scale = (window.width() / width).min(window.height() / height);
            transform.scale = Vec3::new(scale, scale, 1.0);
            sprite.custom_size = Some(Vec2::new(width, height));
            transform.translation = Vec3::ZERO;
        }
    }
}

fn cleanup_loading_scene(mut commands: Commands, query: Query<Entity, With<LoadingSceneRoot>>) {
    if let Ok(entity) = query.single() {
        commands.entity(entity).despawn();
    }
    commands.remove_resource::<LoadingTimer>();
}
