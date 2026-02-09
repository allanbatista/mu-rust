pub use crate::scene_runtime::scene_loader::{
    CameraTourData, HeightmapData, ObjectProperties, SceneObjectDef, SceneObjectsData,
    TerrainConfig, TerrainMapData, TerrainMapSample, TerrainTextureSlotsData,
};
use bevy::gltf::Gltf;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// Marker applied to every entity owned by the runtime scene lifecycle.
#[derive(Component)]
pub struct RuntimeSceneEntity;

/// Marker for on-screen debug overlay UI elements.
#[derive(Component)]
pub struct DebugOverlayElement;

// ============================================================================
// TERRAIN COMPONENTS
// ============================================================================

/// Component marking an entity as terrain
#[derive(Component)]
pub struct Terrain;

// ============================================================================
// SCENE OBJECT COMPONENTS
// ============================================================================

/// Component marking a scene object
#[derive(Component)]
pub struct SceneObject;

/// Animation metadata for scene objects spawned from GLB scenes.
#[derive(Component, Clone)]
pub struct SceneObjectAnimationSource {
    pub glb_asset_path: String,
    pub gltf_handle: Handle<Gltf>,
    pub playback_speed: f32,
}

/// Marker to avoid re-initializing the same animation player repeatedly.
#[derive(Component)]
pub struct SceneObjectAnimationInitialized;

// ============================================================================
// PARTICLE COMPONENTS
// ============================================================================

/// Component for particle emitter
#[derive(Component)]
pub struct ParticleEmitter {
    pub config: ParticleEmitterConfig,
    pub active: bool,
    pub particles: Vec<Particle>,
    pub spawn_timer: Timer,
}

#[derive(Clone)]
pub struct ParticleEmitterConfig {
    pub lifetime_range: (f32, f32),
    pub initial_velocity: Vec3,
    pub velocity_variance: Vec3,
}

pub struct Particle {
    pub position: Vec3,
    pub velocity: Vec3,
    pub lifetime: f32,
    pub max_lifetime: f32,
}

/// Particle definitions loaded from JSON
#[derive(Asset, TypePath, Serialize, Deserialize, Clone)]
pub struct ParticleDefinitions {
    pub emitters: std::collections::HashMap<String, ParticleEmitterDef>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ParticleEmitterDef {
    pub texture: String,
    pub spawn_rate: f32,
    pub lifetime: [f32; 2],
    pub initial_velocity: [f32; 3],
    pub velocity_variance: [f32; 3],
    pub scale: [f32; 2],
    #[serde(default)]
    pub scale_variance: f32,
    pub color: [f32; 4],
    pub color_fade: [f32; 4],
    pub blend_mode: String,
    pub rotation_speed: Option<f32>,
}

// ============================================================================
// LIGHTING COMPONENTS
// ============================================================================

/// Component for dynamic point lights
#[derive(Component, Clone)]
pub struct DynamicLight {
    pub color: Color,
    pub intensity: f32,
    pub range: f32,
    pub flicker: Option<FlickerParams>,
}

#[derive(Clone)]
pub struct FlickerParams {
    pub min_intensity: f32,
    pub max_intensity: f32,
    pub speed: f32,
}

// ============================================================================
// CAMERA COMPONENTS
// ============================================================================

/// Component for camera tour controller
#[derive(Component)]
pub struct CameraTour {
    pub waypoints: Vec<CameraWaypoint>,
    pub current_index: usize,
    pub next_index: usize,
    pub progress: f32,
    pub speed: f32,
    pub active: bool,
    pub loop_enabled: bool,
}

#[derive(Clone, Debug)]
pub struct CameraWaypoint {
    pub position: Vec3,
    pub look_at: Vec3,
    pub move_acceleration: f32,
    pub distance_level: f32,
    pub delay: f32,
}

/// Component for camera tour delay state
#[derive(Component)]
pub struct CameraTourState {
    pub delay_timer: Option<Timer>,
}

// ============================================================================
// BOID COMPONENTS
// ============================================================================

/// Component for boid entities (eagles)
#[derive(Component)]
pub struct Boid {
    pub spawn_point: Vec3,
    pub animation_timer: Timer,
}

/// Component for boid flight pattern
#[derive(Component)]
pub struct BoidFlightPattern {
    pub pattern_type: FlightPattern,
    pub time: f32,
}

#[derive(Clone)]
pub enum FlightPattern {
    Circular { radius: f32, speed: f32 },
}
