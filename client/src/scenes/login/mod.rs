use crate::scene_runtime::components::{
    ParticleDefinitions, ParticleEmitterDef, RuntimeSceneEntity,
};
use crate::scene_runtime::registration::register_scene_runtime;
use crate::scene_runtime::state::RuntimeSceneAssets;
use crate::scene_runtime::systems::ParticleDefinitionsLoader;
use crate::world::{WorldId, WorldRequest};
use bevy::prelude::*;
use bevy::state::prelude::{OnEnter, OnExit};
use std::collections::HashMap;

use super::SceneBuilder;

const DEFAULT_LOGIN_WORLD: u32 = 73;
const DEFAULT_FIRE_PARTICLE_TEXTURE: &str = "data/Skill/bons_particle.png";
const DEFAULT_CLOUD_PARTICLE_TEXTURE: &str = "data/Effect/hart_particle02.png";

fn login_world_name() -> String {
    match std::env::var("MU_LOGIN_WORLD") {
        Ok(raw_world) => {
            let trimmed = raw_world.trim();
            if let Some(stripped) = trimmed.strip_prefix("World") {
                if let Ok(parsed) = stripped.parse::<u32>() {
                    return format!("World{}", parsed);
                }
            } else if let Ok(parsed) = trimmed.parse::<u32>() {
                return format!("World{}", parsed);
            }

            warn!(
                "Invalid MU_LOGIN_WORLD='{}'; using default World{}",
                raw_world, DEFAULT_LOGIN_WORLD
            );
            format!("World{}", DEFAULT_LOGIN_WORLD)
        }
        Err(_) => format!("World{}", DEFAULT_LOGIN_WORLD),
    }
}

/// Login scene wiring using the shared 3D runtime systems.
pub struct LoginScene;

impl SceneBuilder for LoginScene {
    fn register(app: &mut App) {
        app.init_asset::<ParticleDefinitions>()
            .init_asset_loader::<ParticleDefinitionsLoader>()
            .add_systems(OnEnter(crate::AppState::Mock), setup_login_scene)
            .add_systems(OnExit(crate::AppState::Mock), cleanup_login_scene);
        register_scene_runtime(app, crate::AppState::Mock);
    }
}

#[derive(Component)]
struct LoginSceneRoot;

fn default_particle_definitions() -> ParticleDefinitions {
    let mut emitters = HashMap::new();

    emitters.insert(
        "fire_orange".to_string(),
        ParticleEmitterDef {
            texture: DEFAULT_FIRE_PARTICLE_TEXTURE.to_string(),
            spawn_rate: 18.0,
            lifetime: [0.6, 1.4],
            initial_velocity: [0.0, 14.0, 0.0],
            velocity_variance: [2.5, 5.0, 2.5],
            scale: [0.45, 0.95],
            scale_variance: 0.15,
            color: [1.0, 0.55, 0.15, 0.95],
            color_fade: [0.95, 0.1, 0.0, 0.0],
            blend_mode: "additive".to_string(),
            rotation_speed: Some(0.8),
        },
    );

    emitters.insert(
        "cloud".to_string(),
        ParticleEmitterDef {
            texture: DEFAULT_CLOUD_PARTICLE_TEXTURE.to_string(),
            spawn_rate: 8.0,
            lifetime: [1.2, 2.8],
            initial_velocity: [0.0, 1.2, 0.0],
            velocity_variance: [0.8, 0.6, 0.8],
            scale: [0.8, 1.8],
            scale_variance: 0.2,
            color: [0.92, 0.92, 0.95, 0.55],
            color_fade: [0.92, 0.92, 0.95, 0.0],
            blend_mode: "alpha".to_string(),
            rotation_speed: Some(0.25),
        },
    );

    ParticleDefinitions { emitters }
}

fn setup_login_scene(
    mut commands: Commands,
    mut particle_definitions_assets: ResMut<Assets<ParticleDefinitions>>,
    mut world_requests: EventWriter<WorldRequest>,
) {
    let world_name = login_world_name();
    info!("Setting up login scene from {}", world_name);

    world_requests.send(WorldRequest(WorldId::Login));

    let particle_defs = particle_definitions_assets.add(default_particle_definitions());
    commands.insert_resource(RuntimeSceneAssets {
        world_name,
        world: None,
        particle_defs,
        loaded: false,
    });

    commands.spawn((LoginSceneRoot, RuntimeSceneEntity, SpatialBundle::default()));
}

fn cleanup_login_scene(mut commands: Commands, query: Query<Entity, With<RuntimeSceneEntity>>) {
    info!("Cleaning up login scene");

    for entity in &query {
        commands.entity(entity).despawn_recursive();
    }

    commands.remove_resource::<RuntimeSceneAssets>();
}
