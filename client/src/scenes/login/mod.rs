//! # Login Scene Module
//!
//! Manages the login/character selection scene using the WorldMap enum from the common crate.
//!
//! ## Setting Login World
//!
//! The login world can be configured via the `MU_LOGIN_WORLD` environment variable:
//!
//! ```bash
//! # Using world ID (numeric)
//! MU_LOGIN_WORLD=55    # Original login scene (WD_55LOGINSCENE)
//! MU_LOGIN_WORLD=73    # New login scene v1 (WD_73NEW_LOGIN_SCENE)
//! MU_LOGIN_WORLD=77    # New login scene v2 (WD_77NEW_LOGIN_SCENE)
//! ```
//!
//! If not set, defaults to `WorldMap::LoginScene` (ID 55).

use crate::scene_runtime::components::{
    ParticleDefinitions, ParticleEmitterDef, RuntimeSceneEntity,
};
use crate::scene_runtime::registration::register_scene_runtime;
use crate::scene_runtime::state::RuntimeSceneAssets;
use crate::scene_runtime::systems::ParticleDefinitionsLoader;
use crate::world::{WorldId, WorldRequest};
use bevy::prelude::*;
use bevy::state::prelude::{OnEnter, OnExit};
use common::WorldMap;
use std::collections::HashMap;

use super::SceneBuilder;

// Default login world (can be changed via MU_LOGIN_WORLD environment variable)
// Valid login worlds are: 55, 73, 77 (LoginScene, NewLoginScene1, NewLoginScene2)
const DEFAULT_LOGIN_WORLD: WorldMap = WorldMap::Lorencia;
const DEFAULT_FIRE_PARTICLE_TEXTURE: &str = "data/effect/flame_chrom2.png";
const DEFAULT_CLOUD_PARTICLE_TEXTURE: &str = "data/effect/hart_particle02.png";

/// Gets the login world from environment variable or returns the default.
///
/// Accepts:
/// - `MU_LOGIN_WORLD=55` or `MU_LOGIN_WORLD=LoginScene`
/// - `MU_LOGIN_WORLD=73` or `MU_LOGIN_WORLD=NewLoginScene1`
/// - `MU_LOGIN_WORLD=77` or `MU_LOGIN_WORLD=NewLoginScene2`s
fn get_login_world() -> WorldMap {
    match std::env::var("MU_LOGIN_WORLD") {
        Ok(raw_world) => {
            let trimmed = raw_world.trim();

            // Try to parse as world ID (u8)
            if let Ok(id) = trimmed.parse::<u8>() {
                if let Some(map) = WorldMap::from_id(id) {
                    // Validate that it's a login scene
                    if map.is_login_scene() {
                        info!(
                            "Using login world from MU_LOGIN_WORLD: {} (ID: {})",
                            map.name(),
                            id
                        );
                        return map;
                    } else {
                        warn!(
                            "MU_LOGIN_WORLD={} (ID: {}) is not a login scene; using default {}",
                            id,
                            id,
                            DEFAULT_LOGIN_WORLD.name()
                        );
                    }
                } else {
                    warn!(
                        "MU_LOGIN_WORLD={} is not a valid world ID; using default {}",
                        id,
                        DEFAULT_LOGIN_WORLD.name()
                    );
                }
            } else {
                warn!(
                    "MU_LOGIN_WORLD='{}' is not a valid world ID; using default {}",
                    raw_world,
                    DEFAULT_LOGIN_WORLD.name()
                );
            }

            DEFAULT_LOGIN_WORLD
        }
        Err(_) => DEFAULT_LOGIN_WORLD,
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

    emitters.insert(
        "losttower_fire".to_string(),
        ParticleEmitterDef {
            texture: "data/effect/flame_chrom2.png".to_string(),
            spawn_rate: 14.0,
            lifetime: [0.45, 1.15],
            initial_velocity: [0.0, 20.0, 0.0],
            velocity_variance: [3.5, 8.0, 3.5],
            scale: [0.65, 1.35],
            scale_variance: 0.2,
            color: [1.0, 0.58, 0.25, 0.95],
            color_fade: [0.95, 0.2, 0.06, 0.0],
            blend_mode: "additive".to_string(),
            rotation_speed: Some(1.4),
        },
    );

    emitters.insert(
        "losttower_arcane".to_string(),
        ParticleEmitterDef {
            texture: "data/skill/light01.png".to_string(),
            spawn_rate: 8.0,
            lifetime: [0.5, 1.25],
            initial_velocity: [0.0, 8.5, 0.0],
            velocity_variance: [1.8, 4.0, 1.8],
            scale: [0.7, 1.35],
            scale_variance: 0.18,
            color: [0.45, 0.78, 1.0, 0.9],
            color_fade: [0.2, 0.38, 1.0, 0.0],
            blend_mode: "additive".to_string(),
            rotation_speed: Some(1.1),
        },
    );

    emitters.insert(
        "losttower_smoke".to_string(),
        ParticleEmitterDef {
            texture: "data/effect/hart_particle02.png".to_string(),
            spawn_rate: 5.5,
            lifetime: [1.25, 2.8],
            initial_velocity: [0.0, 6.0, 0.0],
            velocity_variance: [1.2, 2.2, 1.2],
            scale: [0.75, 1.9],
            scale_variance: 0.15,
            color: [0.78, 0.74, 0.7, 0.4],
            color_fade: [0.65, 0.62, 0.58, 0.0],
            blend_mode: "alpha".to_string(),
            rotation_speed: Some(0.35),
        },
    );

    ParticleDefinitions { emitters }
}

fn setup_login_scene(
    mut commands: Commands,
    mut particle_definitions_assets: ResMut<Assets<ParticleDefinitions>>,
    mut world_requests: EventWriter<WorldRequest>,
) {
    let login_world = get_login_world();
    let world_name = format!("world_{}", login_world as u8);

    info!(
        "Setting up login scene: {} (ID: {})",
        login_world.name(),
        login_world as u8
    );

    // Send world request with the login world map
    world_requests.send(WorldRequest(WorldId::Login(login_world)));

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
