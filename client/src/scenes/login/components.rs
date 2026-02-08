use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// Marker applied to every entity owned by the login scene lifecycle.
#[derive(Component)]
pub struct LoginSceneEntity;

// ============================================================================
// TERRAIN COMPONENTS
// ============================================================================

/// Component marking an entity as terrain
#[derive(Component)]
pub struct Terrain {
    pub width: u32,
    pub height: u32,
}

/// Terrain configuration loaded from JSON
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

/// Heightmap data loaded from JSON
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

// ============================================================================
// SCENE OBJECT COMPONENTS
// ============================================================================

/// Component marking a scene object
#[derive(Component)]
pub struct SceneObject {
    pub id: String,
    pub object_type: u32,
}

/// Scene objects data loaded from JSON
#[derive(Asset, TypePath, Serialize, Deserialize, Clone)]
pub struct SceneObjectsData {
    pub objects: Vec<SceneObjectDef>,
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
    pub texture: Handle<Image>,
    pub spawn_rate: f32,
    pub lifetime_range: (f32, f32),
    pub initial_velocity: Vec3,
    pub velocity_variance: Vec3,
    pub scale_range: (f32, f32),
    pub scale_variance: f32,
    pub color_start: Color,
    pub color_end: Color,
    pub blend_mode: ParticleBlendMode,
    pub rotation_speed: Option<f32>,
}

#[derive(Clone, Copy)]
pub enum ParticleBlendMode {
    Additive,
    Alpha,
}

pub struct Particle {
    pub position: Vec3,
    pub velocity: Vec3,
    pub lifetime: f32,
    pub max_lifetime: f32,
    pub scale: f32,
    pub rotation: f32,
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
#[derive(Component)]
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
    pub blend_distance: f32,
}

#[derive(Clone, Debug)]
pub struct CameraWaypoint {
    pub index: u32,
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

/// Camera tour data loaded from JSON
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

// ============================================================================
// BOID COMPONENTS
// ============================================================================

/// Component for boid entities (eagles)
#[derive(Component)]
pub struct Boid {
    pub boid_type: BoidType,
    pub velocity: Vec3,
    pub flight_radius: f32,
    pub flight_height: f32,
    pub spawn_point: Vec3,
    pub animation_timer: Timer,
}

#[derive(Clone, Copy)]
pub enum BoidType {
    Eagle,
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
    Patrol { points: Vec<Vec3>, current: usize },
}
