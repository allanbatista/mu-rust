use crate::scenes::login::components::*;
use bevy::asset::io::Reader;
use bevy::asset::{AssetLoader, AsyncReadExt, LoadContext};
use bevy::prelude::*;
use thiserror::Error;

// ============================================================================
// ASSET LOADING SYSTEM
// ============================================================================

/// System to check if all login assets are loaded
pub fn load_login_assets(
    mut assets: ResMut<super::super::LoginSceneAssets>,
    terrain_configs: Res<Assets<TerrainConfig>>,
    heightmaps: Res<Assets<HeightmapData>>,
    scene_objects: Res<Assets<SceneObjectsData>>,
    particle_defs: Res<Assets<ParticleDefinitions>>,
    camera_tours: Res<Assets<CameraTourData>>,
) {
    if assets.loaded {
        return;
    }

    // Check if all assets are loaded
    let all_loaded = terrain_configs.get(&assets.terrain_config).is_some()
        && heightmaps.get(&assets.heightmap).is_some()
        && scene_objects.get(&assets.scene_objects).is_some()
        && particle_defs.get(&assets.particle_defs).is_some()
        && camera_tours.get(&assets.camera_tour).is_some();

    if all_loaded {
        info!("All login scene assets loaded successfully");
        assets.loaded = true;
    }
}

// ============================================================================
// TERRAIN CONFIG LOADER
// ============================================================================

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
        &["terrain_config.json", "json"]
    }
}

// ============================================================================
// HEIGHTMAP LOADER
// ============================================================================

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
        &["heightmap.json", "json"]
    }
}

// ============================================================================
// SCENE OBJECTS LOADER
// ============================================================================

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
        &["scene_objects.json", "json"]
    }
}

// ============================================================================
// PARTICLE DEFINITIONS LOADER
// ============================================================================

#[derive(Default)]
pub struct ParticleDefinitionsLoader;

#[derive(Debug, Error)]
pub enum ParticleDefinitionsLoaderError {
    #[error("Could not load particle definitions: {0}")]
    Io(#[from] std::io::Error),
    #[error("Could not parse JSON: {0}")]
    JsonError(#[from] serde_json::Error),
}

impl AssetLoader for ParticleDefinitionsLoader {
    type Asset = ParticleDefinitions;
    type Settings = ();
    type Error = ParticleDefinitionsLoaderError;

    async fn load<'a>(
        &'a self,
        reader: &'a mut Reader<'_>,
        _settings: &'a (),
        _load_context: &'a mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let defs = serde_json::from_slice::<ParticleDefinitions>(&bytes)?;
        Ok(defs)
    }

    fn extensions(&self) -> &[&str] {
        &["particle_definitions.json", "json"]
    }
}

// ============================================================================
// CAMERA TOUR LOADER
// ============================================================================

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
        &["camera_tour.json", "json"]
    }
}
