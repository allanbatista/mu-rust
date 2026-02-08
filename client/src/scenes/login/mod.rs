mod components;
mod systems;

pub use components::*;
pub use systems::*;

use crate::world::{WorldId, WorldRequest};
use bevy::prelude::*;
use bevy::state::prelude::{in_state, OnEnter, OnExit};

use super::SceneBuilder;

/// LoginScene implements the 3D login scene with terrain, objects, particles, and camera tour
pub struct LoginScene;

impl SceneBuilder for LoginScene {
    fn register(app: &mut App) {
        app
            // Register custom asset loaders
            .init_asset::<TerrainConfig>()
            .init_asset::<HeightmapData>()
            .init_asset::<SceneObjectsData>()
            .init_asset::<ParticleDefinitions>()
            .init_asset::<CameraTourData>()
            .init_asset_loader::<TerrainConfigLoader>()
            .init_asset_loader::<HeightmapLoader>()
            .init_asset_loader::<SceneObjectsLoader>()
            .init_asset_loader::<ParticleDefinitionsLoader>()
            .init_asset_loader::<CameraTourLoader>()
            // Register custom materials
            .add_plugins(MaterialPlugin::<TerrainMaterial>::default())
            // Setup systems
            .add_systems(OnEnter(crate::AppState::Mock), setup_login_scene)
            .add_systems(
                Update,
                (
                    // Asset loading
                    load_login_assets,
                    // Terrain
                    spawn_terrain_when_ready,
                    // Objects
                    spawn_scene_objects_when_ready,
                    // Camera setup
                    setup_camera_tour,
                    // Particles
                    update_particle_emitters,
                    // Lighting
                    spawn_dynamic_lights,
                    update_dynamic_lights,
                    // Camera
                    update_camera_tour,
                    // Boids
                    update_boids,
                )
                    .run_if(in_state(crate::AppState::Mock)),
            )
            .add_systems(OnExit(crate::AppState::Mock), cleanup_login_scene);
    }
}

/// Marker component for login scene root entity
#[derive(Component)]
pub struct LoginSceneRoot;

/// Resource to track login scene asset loading state
#[derive(Resource, Default)]
pub struct LoginSceneAssets {
    pub terrain_config: Handle<TerrainConfig>,
    pub heightmap: Handle<HeightmapData>,
    pub scene_objects: Handle<SceneObjectsData>,
    pub particle_defs: Handle<ParticleDefinitions>,
    pub camera_tour: Handle<CameraTourData>,
    pub loaded: bool,
}

/// Setup function called when entering login scene
fn setup_login_scene(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut world_requests: EventWriter<WorldRequest>,
) {
    info!("Setting up login scene");

    // Request login world
    world_requests.send(WorldRequest(WorldId::Login));

    // Load all asset definitions
    let assets = LoginSceneAssets {
        terrain_config: asset_server.load("data/World73/terrain_config.json"),
        heightmap: asset_server.load("data/World73/heightmap.json"),
        scene_objects: asset_server.load("data/World73/scene_objects.json"),
        particle_defs: asset_server.load("data/particle_definitions.json"),
        camera_tour: asset_server.load("data/World73/camera_tour.json"),
        loaded: false,
    };

    commands.insert_resource(assets);

    // Spawn scene root
    commands.spawn((LoginSceneRoot, SpatialBundle::default()));
}

/// Cleanup function called when exiting login scene
fn cleanup_login_scene(
    mut commands: Commands,
    query: Query<Entity, With<LoginSceneRoot>>,
) {
    info!("Cleaning up login scene");

    for entity in &query {
        commands.entity(entity).despawn_recursive();
    }

    commands.remove_resource::<LoginSceneAssets>();
}
