use crate::scene_runtime::components::ParticleDefinitions;
use crate::scene_runtime::scene_loader::LoadedSceneWorld;
use bevy::prelude::*;

/// Resource tracking world and dependent scene assets loaded by the runtime.
#[derive(Resource)]
pub struct RuntimeSceneAssets {
    pub world_name: String,
    pub world: Option<LoadedSceneWorld>,
    pub particle_defs: Handle<ParticleDefinitions>,
    pub loaded: bool,
}
