use crate::scene_runtime::components::*;
use crate::scene_runtime::scene_loader::SceneLoader;
use crate::scene_runtime::state::RuntimeSceneAssets;
use bevy::asset::io::Reader;
use bevy::asset::{AssetLoader, AsyncReadExt, LoadContext};
use bevy::prelude::*;
use thiserror::Error;

// ============================================================================
// ASSET LOADING SYSTEM
// ============================================================================

/// System to check if all runtime scene assets are loaded.
/// It requests a world by name through the shared scene loader and only marks
/// the scene as ready when world data + scene-specific assets are available.
pub fn load_scene_runtime_assets(
    mut assets: ResMut<RuntimeSceneAssets>,
    mut scene_loader: ResMut<SceneLoader>,
    asset_server: Res<AssetServer>,
    terrain_configs: Res<Assets<TerrainConfig>>,
    heightmaps: Res<Assets<HeightmapData>>,
    terrain_maps: Res<Assets<TerrainMapData>>,
    scene_objects: Res<Assets<SceneObjectsData>>,
    camera_tours: Res<Assets<CameraTourData>>,
    particle_defs: Res<Assets<ParticleDefinitions>>,
) {
    if assets.loaded {
        return;
    }

    let Some(world) = scene_loader.world(
        &assets.world_name,
        &asset_server,
        &terrain_configs,
        &heightmaps,
        &terrain_maps,
        &scene_objects,
        &camera_tours,
    ) else {
        return;
    };

    if particle_defs.get(&assets.particle_defs).is_none() {
        return;
    }

    info!(
        "All runtime scene assets loaded successfully for {}",
        world.world_name
    );
    assets.world = Some(world);
    assets.loaded = true;
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
        &["particle_definitions.json"]
    }
}
