use bevy::prelude::*;

#[derive(SystemSet, Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum GameplayPipelineSet {
    AssetLoad,
    WorldSpawn,
    WorldSimulate,
    EffectsSimulate,
    Lighting,
    Camera,
    UiSync,
}
