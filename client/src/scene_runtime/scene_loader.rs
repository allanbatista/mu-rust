use bevy::asset::io::Reader;
use bevy::asset::{AssetLoader, AsyncReadExt, LoadContext};
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use thiserror::Error;

const TERRAIN_CONFIG_FILE: &str = "terrain_config.json";
const TERRAIN_HEIGHT_FILE: &str = "terrain_height.json";
const TERRAIN_MAP_FILE: &str = "terrain_map.json";
const TERRAIN_TEXTURE_SLOTS_FILE: &str = "terrain_texture_slots.json";
const SCENE_OBJECTS_FILE: &str = "scene_objects.json";
const CAMERA_TOUR_FILE: &str = "camera_tour.json";
const MAP_VFX_FILE: &str = "map_vfx.json";
const SCENE_OBJECTS_FILE_OVERRIDE_ENV: &str = "MU_SCENE_OBJECTS_FILE";
const DISABLE_MAP_VFX_ENV: &str = "MU_DISABLE_MAP_VFX";
const CLIENT_ASSETS_ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../assets");

#[derive(Asset, TypePath, Serialize, Deserialize, Clone, Debug)]
pub struct TerrainConfig {
    pub size: TerrainSize,
    #[serde(default = "default_height_multiplier")]
    pub height_multiplier: f32,
    #[serde(default = "default_legacy_terrain_scale")]
    pub legacy_terrain_scale: f32,
    #[serde(default = "default_terrain_ambient_light")]
    pub ambient_light: f32,
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

fn default_terrain_ambient_light() -> f32 {
    0.25
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

#[derive(Asset, TypePath, Serialize, Deserialize, Clone, Debug)]
pub struct TerrainMapData {
    #[serde(default)]
    pub header: TerrainMapHeader,
    #[serde(default = "default_terrain_size")]
    pub terrain_size: u32,
    pub layer1: Vec<Vec<u8>>,
    pub layer2: Vec<Vec<u8>>,
    pub alpha: Vec<Vec<u8>>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct TerrainMapHeader {
    #[serde(default)]
    pub version: u8,
    #[serde(default)]
    pub map_number: u8,
}

fn default_terrain_size() -> u32 {
    256
}

impl TerrainMapData {
    pub fn width(&self) -> usize {
        self.layer1.first().map(Vec::len).unwrap_or(0)
    }

    pub fn height(&self) -> usize {
        self.layer1.len()
    }

    pub fn sample(&self, x: usize, z: usize) -> Option<TerrainMapSample> {
        let layer1 = self.layer1.get(z)?.get(x).copied()?;
        let layer2 = self.layer2.get(z)?.get(x).copied()?;
        let alpha = self.alpha.get(z)?.get(x).copied()?;
        Some(TerrainMapSample {
            layer1,
            layer2,
            alpha,
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub struct TerrainMapSample {
    pub layer1: u8,
    pub layer2: u8,
    pub alpha: u8,
}

#[derive(Asset, TypePath, Serialize, Deserialize, Clone, Debug, Default)]
pub struct TerrainTextureSlotsData {
    #[serde(default)]
    pub world: Option<u32>,
    #[serde(default)]
    pub slots: HashMap<u16, String>,
}

impl TerrainTextureSlotsData {
    pub fn path_for_slot(&self, slot: u8) -> Option<&str> {
        self.slots
            .get(&(slot as u16))
            .map(String::as_str)
            .filter(|path| !path.trim().is_empty())
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
    #[serde(default)]
    pub generated_placeholder: bool,
    #[serde(default)]
    pub reason: Option<String>,
    #[serde(default)]
    pub rotation_yaw_offset_degrees: Option<f32>,
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
    pub animation_speed: Option<f32>,
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

#[derive(Asset, TypePath, Serialize, Deserialize, Clone, Debug, Default)]
pub struct MapVfxProfile {
    #[serde(default)]
    pub object_overrides: Vec<MapVfxObjectOverride>,
    #[serde(default)]
    pub object_sprites: Vec<MapVfxObjectSprite>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MapVfxObjectOverride {
    pub object_type: u32,
    #[serde(default)]
    pub particle_emitter: Option<String>,
    #[serde(default)]
    pub light_color: Option<[f32; 3]>,
    #[serde(default)]
    pub light_intensity: Option<f32>,
    #[serde(default)]
    pub light_range: Option<f32>,
    #[serde(default)]
    pub flicker: Option<MapVfxFlicker>,
    #[serde(default = "default_spawn_stride")]
    pub spawn_stride: u32,
    #[serde(default)]
    pub max_instances: Option<u32>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MapVfxObjectSprite {
    pub object_type: u32,
    pub texture: String,
    #[serde(default = "default_sprite_size")]
    pub size: f32,
    #[serde(default = "default_sprite_color")]
    pub color: [f32; 4],
    #[serde(default)]
    pub z_offset: f32,
    #[serde(default)]
    pub spin_speed: f32,
    #[serde(default)]
    pub pulse: Option<MapVfxPulse>,
    #[serde(default)]
    pub max_distance: Option<f32>,
    #[serde(default)]
    pub blend_mode: MapVfxBlendMode,
    #[serde(default = "default_spawn_stride")]
    pub spawn_stride: u32,
    #[serde(default)]
    pub max_instances: Option<u32>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MapVfxFlicker {
    pub min_intensity: f32,
    pub max_intensity: f32,
    pub speed: f32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MapVfxPulse {
    #[serde(default = "default_pulse_amplitude")]
    pub amplitude: f32,
    #[serde(default = "default_pulse_speed")]
    pub speed: f32,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum MapVfxBlendMode {
    #[default]
    Alpha,
    Additive,
}

fn default_spawn_stride() -> u32 {
    1
}

fn default_sprite_size() -> f32 {
    120.0
}

fn default_sprite_color() -> [f32; 4] {
    [1.0, 1.0, 1.0, 1.0]
}

fn default_pulse_amplitude() -> f32 {
    0.0
}

fn default_pulse_speed() -> f32 {
    1.0
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
pub struct TerrainMapLoader;

#[derive(Debug, Error)]
pub enum TerrainMapLoaderError {
    #[error("Could not load terrain map: {0}")]
    Io(#[from] std::io::Error),
    #[error("Could not parse JSON: {0}")]
    JsonError(#[from] serde_json::Error),
}

impl AssetLoader for TerrainMapLoader {
    type Asset = TerrainMapData;
    type Settings = ();
    type Error = TerrainMapLoaderError;

    async fn load<'a>(
        &'a self,
        reader: &'a mut Reader<'_>,
        _settings: &'a (),
        _load_context: &'a mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let terrain_map = serde_json::from_slice::<TerrainMapData>(&bytes)?;
        Ok(terrain_map)
    }

    fn extensions(&self) -> &[&str] {
        &[TERRAIN_MAP_FILE, "map.json"]
    }
}

#[derive(Default)]
pub struct TerrainTextureSlotsLoader;

#[derive(Debug, Error)]
pub enum TerrainTextureSlotsLoaderError {
    #[error("Could not load terrain texture slots: {0}")]
    Io(#[from] std::io::Error),
    #[error("Could not parse JSON: {0}")]
    JsonError(#[from] serde_json::Error),
}

impl AssetLoader for TerrainTextureSlotsLoader {
    type Asset = TerrainTextureSlotsData;
    type Settings = ();
    type Error = TerrainTextureSlotsLoaderError;

    async fn load<'a>(
        &'a self,
        reader: &'a mut Reader<'_>,
        _settings: &'a (),
        _load_context: &'a mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let slots = serde_json::from_slice::<TerrainTextureSlotsData>(&bytes)?;
        Ok(slots)
    }

    fn extensions(&self) -> &[&str] {
        &[TERRAIN_TEXTURE_SLOTS_FILE]
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

#[derive(Default)]
pub struct MapVfxLoader;

#[derive(Debug, Error)]
pub enum MapVfxLoaderError {
    #[error("Could not load map vfx profile: {0}")]
    Io(#[from] std::io::Error),
    #[error("Could not parse JSON: {0}")]
    JsonError(#[from] serde_json::Error),
}

impl AssetLoader for MapVfxLoader {
    type Asset = MapVfxProfile;
    type Settings = ();
    type Error = MapVfxLoaderError;

    async fn load<'a>(
        &'a self,
        reader: &'a mut Reader<'_>,
        _settings: &'a (),
        _load_context: &'a mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let profile = serde_json::from_slice::<MapVfxProfile>(&bytes)?;
        Ok(profile)
    }

    fn extensions(&self) -> &[&str] {
        &[MAP_VFX_FILE]
    }
}

#[derive(Clone, Debug)]
pub struct LoadedSceneWorld {
    pub world_name: String,
    pub terrain_config: Handle<TerrainConfig>,
    pub heightmap: Handle<HeightmapData>,
    pub terrain_map: Handle<TerrainMapData>,
    pub legacy_terrain_map: Option<Handle<TerrainMapData>>,
    pub terrain_texture_slots: Option<Handle<TerrainTextureSlotsData>>,
    pub scene_objects: Handle<SceneObjectsData>,
    pub camera_tour: Handle<CameraTourData>,
    pub map_vfx: Option<Handle<MapVfxProfile>>,
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
        terrain_maps: &Assets<TerrainMapData>,
        scene_objects: &Assets<SceneObjectsData>,
        camera_tours: &Assets<CameraTourData>,
        map_vfx_profiles: &Assets<MapVfxProfile>,
    ) -> Option<LoadedSceneWorld> {
        let world_name = normalize_world_name(world_name);
        let world = self.ensure_requested(&world_name, asset_server);

        let terrain_map_ready = terrain_maps.get(&world.terrain_map).is_some()
            || world
                .legacy_terrain_map
                .as_ref()
                .and_then(|fallback| terrain_maps.get(fallback))
                .is_some();

        let is_ready = terrain_configs.get(&world.terrain_config).is_some()
            && heightmaps.get(&world.heightmap).is_some()
            && terrain_map_ready
            && scene_objects.get(&world.scene_objects).is_some()
            && camera_tours.get(&world.camera_tour).is_some()
            && world
                .map_vfx
                .as_ref()
                .map(|profile| map_vfx_profiles.get(profile).is_some())
                .unwrap_or(true);

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

        let world_number = world_name
            .strip_prefix("world")
            .and_then(|value| value.parse::<u32>().ok());

        let scene_objects_path = scene_objects_asset_path(world_name);
        if scene_objects_path != world_asset_path(world_name, SCENE_OBJECTS_FILE) {
            info!(
                "Using scene object override for {world_name}: {}={}",
                SCENE_OBJECTS_FILE_OVERRIDE_ENV, scene_objects_path
            );
        }

        let world = LoadedSceneWorld {
            world_name: world_name.to_string(),
            terrain_config: asset_server.load(world_asset_path(world_name, TERRAIN_CONFIG_FILE)),
            heightmap: asset_server.load(world_asset_path(world_name, TERRAIN_HEIGHT_FILE)),
            terrain_map: asset_server.load(world_asset_path(world_name, TERRAIN_MAP_FILE)),
            legacy_terrain_map: world_number.map(|number| {
                asset_server.load(world_asset_path(
                    world_name,
                    &format!("enc_terrain{number}.map.json"),
                ))
            }),
            terrain_texture_slots: Some(
                asset_server.load(world_asset_path(world_name, TERRAIN_TEXTURE_SLOTS_FILE)),
            ),
            scene_objects: asset_server.load(scene_objects_path),
            camera_tour: asset_server.load(world_asset_path(world_name, CAMERA_TOUR_FILE)),
            map_vfx: map_vfx_asset_path(world_name).map(|path| asset_server.load(path)),
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

fn optional_world_asset_path_if_exists(world_name: &str, file_name: &str) -> Option<String> {
    let relative = world_asset_path(world_name, file_name);
    let full_path = Path::new(CLIENT_ASSETS_ROOT).join(&relative);
    if full_path.is_file() {
        Some(relative)
    } else {
        None
    }
}

fn map_vfx_asset_path(world_name: &str) -> Option<String> {
    if disable_map_vfx_from_env() {
        return None;
    }
    optional_world_asset_path_if_exists(world_name, MAP_VFX_FILE)
}

fn disable_map_vfx_from_env() -> bool {
    return true;
    
    let Ok(raw_value) = std::env::var(DISABLE_MAP_VFX_ENV) else {
        return false;
    };

    matches!(
        raw_value.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

fn scene_objects_asset_path(world_name: &str) -> String {
    let default_path = world_asset_path(world_name, SCENE_OBJECTS_FILE);
    let Ok(raw_override) = std::env::var(SCENE_OBJECTS_FILE_OVERRIDE_ENV) else {
        return default_path;
    };

    let override_path = raw_override.trim();
    if override_path.is_empty() {
        return default_path;
    }

    if override_path.contains('/') || override_path.contains('\\') {
        override_path.replace('\\', "/")
    } else {
        world_asset_path(world_name, override_path)
    }
}

pub struct SceneLoaderPlugin;

impl Plugin for SceneLoaderPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SceneLoader>()
            .init_asset::<TerrainConfig>()
            .init_asset::<HeightmapData>()
            .init_asset::<TerrainMapData>()
            .init_asset::<TerrainTextureSlotsData>()
            .init_asset::<SceneObjectsData>()
            .init_asset::<CameraTourData>()
            .init_asset::<MapVfxProfile>()
            .init_asset_loader::<TerrainConfigLoader>()
            .init_asset_loader::<HeightmapLoader>()
            .init_asset_loader::<TerrainMapLoader>()
            .init_asset_loader::<TerrainTextureSlotsLoader>()
            .init_asset_loader::<SceneObjectsLoader>()
            .init_asset_loader::<CameraTourLoader>()
            .init_asset_loader::<MapVfxLoader>();
    }
}
