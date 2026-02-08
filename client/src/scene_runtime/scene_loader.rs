use bevy::asset::io::Reader;
use bevy::asset::{AssetLoader, AsyncReadExt, LoadContext};
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

const TERRAIN_CONFIG_FILE: &str = "terrain_config.json";
const TERRAIN_HEIGHT_FILE: &str = "terrain_height.json";
const SCENE_OBJECTS_FILE: &str = "scene_objects.json";
const CAMERA_TOUR_FILE: &str = "camera_tour.json";

#[derive(Asset, TypePath, Serialize, Deserialize, Clone, Debug)]
pub struct TerrainConfig {
    pub size: TerrainSize,
    #[serde(default = "default_height_multiplier")]
    pub height_multiplier: f32,
    #[serde(default = "default_legacy_terrain_scale")]
    pub legacy_terrain_scale: f32,
    pub texture_layers: Vec<TextureLayer>,
    pub alpha_map: String,
    pub lightmap: String,
}

fn default_height_multiplier() -> f32 {
    1.5
}

fn default_legacy_terrain_scale() -> f32 {
    100.0
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TerrainSize {
    pub width: u32,
    pub depth: u32,
    pub scale: f32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TextureLayer {
    pub id: String,
    pub path: String,
    pub scale: f32,
}

#[derive(Asset, TypePath, Serialize, Deserialize, Clone)]
pub struct HeightmapData {
    pub width: u32,
    pub height: u32,
    pub heights: Vec<Vec<f32>>,
}

impl HeightmapData {
    pub fn get_height(&self, x: usize, z: usize) -> f32 {
        if z < self.heights.len() && x < self.heights[z].len() {
            self.heights[z][x]
        } else {
            0.0
        }
    }
}

#[derive(Asset, TypePath, Serialize, Deserialize, Clone)]
pub struct SceneObjectsData {
    pub objects: Vec<SceneObjectDef>,
    #[serde(default)]
    pub metadata: SceneObjectsMetadata,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct SceneObjectsMetadata {
    #[serde(default)]
    pub rotation_encoding: SceneRotationEncoding,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SceneRotationEncoding {
    #[default]
    #[serde(
        alias = "legacy_swizzled_xyz_degrees",
        alias = "legacy_swizzled_xzy_degrees"
    )]
    LegacySwizzledDegrees,
    #[serde(alias = "mu_anglematrix_degrees", alias = "mu_angles_xyz_degrees")]
    MuAnglesDegrees,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SceneObjectDef {
    pub id: String,
    #[serde(rename = "type")]
    pub object_type: u32,
    pub model: String,
    pub position: [f32; 3],
    pub rotation: [f32; 3],
    pub scale: [f32; 3],
    #[serde(default)]
    pub properties: ObjectProperties,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct ObjectProperties {
    pub model_renderable: Option<bool>,
    pub model_validation_reason: Option<String>,
    pub particle_emitter: Option<String>,
    pub light_color: Option<[f32; 3]>,
    pub light_intensity: Option<f32>,
    pub light_range: Option<f32>,
    pub boid_model: Option<String>,
    pub flight_radius: Option<f32>,
    pub flight_height: Option<f32>,
    pub cast_shadow: Option<bool>,
    pub particle_count: Option<u32>,
    pub particle_scale_multiplier: Option<f32>,
}

#[derive(Asset, TypePath, Serialize, Deserialize, Clone)]
pub struct CameraTourData {
    pub waypoints: Vec<CameraWaypointDef>,
    pub r#loop: bool,
    pub blend_distance: f32,
    pub interpolation: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CameraWaypointDef {
    pub index: u32,
    pub position: [f32; 3],
    pub look_at: [f32; 3],
    pub move_acceleration: f32,
    pub distance_level: f32,
    pub delay: f32,
}

#[derive(Default)]
pub struct TerrainConfigLoader;

#[derive(Debug, Error)]
pub enum TerrainConfigLoaderError {
    #[error("Could not load terrain config: {0}")]
    Io(#[from] std::io::Error),
    #[error("Could not parse JSON: {0}")]
    JsonError(#[from] serde_json::Error),
}

impl AssetLoader for TerrainConfigLoader {
    type Asset = TerrainConfig;
    type Settings = ();
    type Error = TerrainConfigLoaderError;

    async fn load<'a>(
        &'a self,
        reader: &'a mut Reader<'_>,
        _settings: &'a (),
        _load_context: &'a mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let config = serde_json::from_slice::<TerrainConfig>(&bytes)?;
        Ok(config)
    }

    fn extensions(&self) -> &[&str] {
        &[TERRAIN_CONFIG_FILE]
    }
}

#[derive(Default)]
pub struct HeightmapLoader;

#[derive(Debug, Error)]
pub enum HeightmapLoaderError {
    #[error("Could not load heightmap: {0}")]
    Io(#[from] std::io::Error),
    #[error("Could not parse JSON: {0}")]
    JsonError(#[from] serde_json::Error),
}

impl AssetLoader for HeightmapLoader {
    type Asset = HeightmapData;
    type Settings = ();
    type Error = HeightmapLoaderError;

    async fn load<'a>(
        &'a self,
        reader: &'a mut Reader<'_>,
        _settings: &'a (),
        _load_context: &'a mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let heightmap = serde_json::from_slice::<HeightmapData>(&bytes)?;
        Ok(heightmap)
    }

    fn extensions(&self) -> &[&str] {
        &[TERRAIN_HEIGHT_FILE]
    }
}

#[derive(Default)]
pub struct SceneObjectsLoader;

#[derive(Debug, Error)]
pub enum SceneObjectsLoaderError {
    #[error("Could not load scene objects: {0}")]
    Io(#[from] std::io::Error),
    #[error("Could not parse JSON: {0}")]
    JsonError(#[from] serde_json::Error),
}

impl AssetLoader for SceneObjectsLoader {
    type Asset = SceneObjectsData;
    type Settings = ();
    type Error = SceneObjectsLoaderError;

    async fn load<'a>(
        &'a self,
        reader: &'a mut Reader<'_>,
        _settings: &'a (),
        _load_context: &'a mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let objects = serde_json::from_slice::<SceneObjectsData>(&bytes)?;
        Ok(objects)
    }

    fn extensions(&self) -> &[&str] {
        &[SCENE_OBJECTS_FILE]
    }
}

#[derive(Default)]
pub struct CameraTourLoader;

#[derive(Debug, Error)]
pub enum CameraTourLoaderError {
    #[error("Could not load camera tour: {0}")]
    Io(#[from] std::io::Error),
    #[error("Could not parse JSON: {0}")]
    JsonError(#[from] serde_json::Error),
}

impl AssetLoader for CameraTourLoader {
    type Asset = CameraTourData;
    type Settings = ();
    type Error = CameraTourLoaderError;

    async fn load<'a>(
        &'a self,
        reader: &'a mut Reader<'_>,
        _settings: &'a (),
        _load_context: &'a mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let tour = serde_json::from_slice::<CameraTourData>(&bytes)?;
        Ok(tour)
    }

    fn extensions(&self) -> &[&str] {
        &[CAMERA_TOUR_FILE]
    }
}

#[derive(Clone, Debug)]
pub struct LoadedSceneWorld {
    pub world_name: String,
    pub terrain_config: Handle<TerrainConfig>,
    pub heightmap: Handle<HeightmapData>,
    pub scene_objects: Handle<SceneObjectsData>,
    pub camera_tour: Handle<CameraTourData>,
}

enum SceneLoadState {
    Loading(LoadedSceneWorld),
    Ready(LoadedSceneWorld),
}

impl SceneLoadState {
    fn world(&self) -> &LoadedSceneWorld {
        match self {
            SceneLoadState::Loading(world) | SceneLoadState::Ready(world) => world,
        }
    }
}

#[derive(Resource, Default)]
pub struct SceneLoader {
    worlds: HashMap<String, SceneLoadState>,
}

impl SceneLoader {
    pub fn world(
        &mut self,
        world_name: &str,
        asset_server: &AssetServer,
        terrain_configs: &Assets<TerrainConfig>,
        heightmaps: &Assets<HeightmapData>,
        scene_objects: &Assets<SceneObjectsData>,
        camera_tours: &Assets<CameraTourData>,
    ) -> Option<LoadedSceneWorld> {
        let world_name = normalize_world_name(world_name);
        let world = self.ensure_requested(&world_name, asset_server);

        let is_ready = terrain_configs.get(&world.terrain_config).is_some()
            && heightmaps.get(&world.heightmap).is_some()
            && scene_objects.get(&world.scene_objects).is_some()
            && camera_tours.get(&world.camera_tour).is_some();

        if !is_ready {
            return None;
        }

        self.worlds
            .insert(world_name, SceneLoadState::Ready(world.clone()));
        Some(world)
    }

    fn ensure_requested(
        &mut self,
        world_name: &str,
        asset_server: &AssetServer,
    ) -> LoadedSceneWorld {
        if let Some(existing) = self.worlds.get(world_name) {
            return existing.world().clone();
        }

        let world = LoadedSceneWorld {
            world_name: world_name.to_string(),
            terrain_config: asset_server.load(world_asset_path(world_name, TERRAIN_CONFIG_FILE)),
            heightmap: asset_server.load(world_asset_path(world_name, TERRAIN_HEIGHT_FILE)),
            scene_objects: asset_server.load(world_asset_path(world_name, SCENE_OBJECTS_FILE)),
            camera_tour: asset_server.load(world_asset_path(world_name, CAMERA_TOUR_FILE)),
        };

        self.worlds.insert(
            world_name.to_string(),
            SceneLoadState::Loading(world.clone()),
        );
        world
    }
}

pub fn normalize_world_name(raw_world_name: &str) -> String {
    let trimmed = raw_world_name.trim();
    if trimmed.is_empty() {
        return "World1".to_string();
    }

    if let Some(stripped) = trimmed.strip_prefix("World") {
        if let Ok(number) = stripped.parse::<u32>() {
            return format!("world{}", number);
        }
        return trimmed.to_string();
    }

    if let Ok(number) = trimmed.parse::<u32>() {
        return format!("world{}", number);
    }

    trimmed.to_string()
}

fn world_asset_path(world_name: &str, file_name: &str) -> String {
    format!("data/{world_name}/{file_name}")
}

pub struct SceneLoaderPlugin;

impl Plugin for SceneLoaderPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SceneLoader>()
            .init_asset::<TerrainConfig>()
            .init_asset::<HeightmapData>()
            .init_asset::<SceneObjectsData>()
            .init_asset::<CameraTourData>()
            .init_asset_loader::<TerrainConfigLoader>()
            .init_asset_loader::<HeightmapLoader>()
            .init_asset_loader::<SceneObjectsLoader>()
            .init_asset_loader::<CameraTourLoader>();
    }
}
