//! Gameplay scene wiring for runtime world loading.

use crate::scene_runtime::components::{ParticleDefinitions, RuntimeSceneEntity};
use crate::scene_runtime::state::RuntimeSceneAssets;
use crate::scene_runtime::systems::ParticleDefinitionsLoader;
use crate::world::{WorldId, WorldRequest};
use bevy::camera::ClearColorConfig;
use bevy::prelude::*;
use bevy::state::prelude::{OnEnter, OnExit};
use common::WorldMap;
use std::collections::HashMap;

use super::SceneBuilder;

const DEFAULT_GAMEPLAY_WORLD: WorldMap = WorldMap::Lorencia;
const GAMEPLAY_CLEAR_COLOR: Color = Color::srgb(0.1, 0.1, 0.15);

pub struct GameplayScene;

impl SceneBuilder for GameplayScene {
    fn register(app: &mut App) {
        app.init_asset::<ParticleDefinitions>()
            .init_asset_loader::<ParticleDefinitionsLoader>()
            .add_systems(OnEnter(crate::AppState::Gameplay), setup_gameplay_scene)
            .add_systems(OnExit(crate::AppState::Gameplay), cleanup_gameplay_scene);
    }
}

#[derive(Component)]
struct GameplaySceneRoot;

fn get_gameplay_world() -> WorldMap {
    match std::env::var("MU_GAMEPLAY_WORLD") {
        Ok(raw_world) => {
            let trimmed = raw_world.trim();
            if let Ok(id) = trimmed.parse::<u8>() {
                if let Some(map) = WorldMap::from_id(id) {
                    info!(
                        "Using gameplay world from MU_GAMEPLAY_WORLD: {} (ID: {})",
                        map.name(),
                        id
                    );
                    return map;
                }
            }

            warn!(
                "MU_GAMEPLAY_WORLD='{}' is not a valid world ID. Using default {} ({})",
                raw_world,
                DEFAULT_GAMEPLAY_WORLD.name(),
                DEFAULT_GAMEPLAY_WORLD as u8
            );
            DEFAULT_GAMEPLAY_WORLD
        }
        Err(_) => DEFAULT_GAMEPLAY_WORLD,
    }
}

fn default_particle_definitions() -> ParticleDefinitions {
    ParticleDefinitions {
        emitters: HashMap::new(),
    }
}

fn setup_gameplay_scene(
    mut commands: Commands,
    mut particle_definitions_assets: ResMut<Assets<ParticleDefinitions>>,
    mut world_requests: MessageWriter<WorldRequest>,
    mut camera_query: Query<&mut Camera, With<Camera3d>>,
) {
    let gameplay_world = get_gameplay_world();
    let world_name = format!("world_{}", gameplay_world as u8);

    info!(
        "Setting up gameplay scene: {} (ID: {})",
        gameplay_world.name(),
        gameplay_world as u8
    );

    world_requests.write(WorldRequest(WorldId::Game(gameplay_world)));
    for mut camera in &mut camera_query {
        camera.clear_color = ClearColorConfig::Custom(GAMEPLAY_CLEAR_COLOR);
    }

    let particle_defs = particle_definitions_assets.add(default_particle_definitions());
    commands.insert_resource(RuntimeSceneAssets {
        world_name,
        world: None,
        particle_defs,
        loaded: false,
    });

    commands.spawn((GameplaySceneRoot, RuntimeSceneEntity, Transform::default()));
}

fn cleanup_gameplay_scene(
    mut commands: Commands,
    query: Query<Entity, Or<(With<RuntimeSceneEntity>, With<GameplaySceneRoot>)>>,
) {
    info!("Cleaning up gameplay scene");

    for entity in &query {
        commands.entity(entity).try_despawn();
    }

    commands.remove_resource::<RuntimeSceneAssets>();
}
