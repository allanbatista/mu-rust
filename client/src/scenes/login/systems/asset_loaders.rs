use crate::scenes::login::components::*;
use bevy::asset::io::Reader;
use bevy::asset::{AssetLoader, AsyncReadExt, LoadContext};
use bevy::prelude::*;
use serde::Deserialize;
use thiserror::Error;

const TERRAIN_DIMENSION: usize = 256;
const OZB_PREFIX_BYTES: usize = 4;
const LEGACY_BMP_HEADER_BYTES: usize = 1080;
const TERRAIN_SAMPLE_COUNT: usize = TERRAIN_DIMENSION * TERRAIN_DIMENSION;

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
    #[error("Could not parse legacy OZB terrain format: {0}")]
    OzbFormat(String),
    #[error("Invalid legacy heightmap format: {0}")]
    InvalidFormat(String),
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

        if let Some(heightmap) = parse_legacy_ozb_heightmap(&bytes)? {
            return Ok(heightmap);
        }

        if let Ok(heightmap) = serde_json::from_slice::<HeightmapData>(&bytes) {
            return Ok(heightmap);
        }

        // Fallback for converter output already present in the repository.
        // Some JSON dumps store heights as u8 (0..255) in `terrain_data`.
        let legacy = serde_json::from_slice::<LegacyTerrainData>(&bytes)?;
        let (width, height) = validate_legacy_rows(&legacy.terrain_data)?;
        let heights = legacy
            .terrain_data
            .into_iter()
            .map(|row| row.into_iter().map(f32::from).collect())
            .collect();

        Ok(HeightmapData {
            width: width as u32,
            height: height as u32,
            heights,
        })
    }

    fn extensions(&self) -> &[&str] {
        &["heightmap.json", "json", "ozb"]
    }
}

#[derive(Deserialize)]
struct LegacyTerrainData {
    terrain_data: Vec<Vec<u8>>,
}

fn validate_legacy_rows(rows: &[Vec<u8>]) -> Result<(usize, usize), HeightmapLoaderError> {
    let height = rows.len();
    if height == 0 {
        return Err(HeightmapLoaderError::InvalidFormat(
            "terrain_data is empty".to_string(),
        ));
    }

    let width = rows[0].len();
    if width == 0 {
        return Err(HeightmapLoaderError::InvalidFormat(
            "terrain_data first row is empty".to_string(),
        ));
    }

    if rows.iter().any(|row| row.len() != width) {
        return Err(HeightmapLoaderError::InvalidFormat(
            "terrain_data rows must all have the same width".to_string(),
        ));
    }

    Ok((width, height))
}

fn parse_legacy_ozb_heightmap(bytes: &[u8]) -> Result<Option<HeightmapData>, HeightmapLoaderError> {
    let required = OZB_PREFIX_BYTES + LEGACY_BMP_HEADER_BYTES + TERRAIN_SAMPLE_COUNT;
    if bytes.len() < required {
        return Ok(None);
    }

    let payload = &bytes[OZB_PREFIX_BYTES..];
    if payload.len() < LEGACY_BMP_HEADER_BYTES + TERRAIN_SAMPLE_COUNT {
        return Err(HeightmapLoaderError::OzbFormat(
            "payload shorter than expected terrain format".to_string(),
        ));
    }

    if &payload[..2] != b"BM" {
        return Ok(None);
    }

    let samples = &payload[LEGACY_BMP_HEADER_BYTES..LEGACY_BMP_HEADER_BYTES + TERRAIN_SAMPLE_COUNT];
    let heights = samples
        .chunks_exact(TERRAIN_DIMENSION)
        .map(|row| row.iter().map(|sample| f32::from(*sample)).collect())
        .collect();

    Ok(Some(HeightmapData {
        width: TERRAIN_DIMENSION as u32,
        height: TERRAIN_DIMENSION as u32,
        heights,
    }))
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
