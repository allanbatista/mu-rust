use bevy::prelude::*;

#[derive(SystemSet, Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum SceneRenderPipeline {
    Load,
    Spawn,
    Simulate,
    Lighting,
    Camera,
}
