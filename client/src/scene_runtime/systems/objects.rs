use crate::scene_runtime::components::*;
use crate::scene_runtime::scene_loader::SceneRotationEncoding;
use crate::scene_runtime::state::RuntimeSceneAssets;
use crate::scene_runtime::transforms::scene_object_rotation_to_quat;
use bevy::ecs::system::EntityCommands;
use bevy::math::primitives::Cuboid;
use bevy::prelude::*;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

/// Marker component to track if scene objects have been spawned
#[derive(Component)]
pub struct SceneObjectsSpawned;

#[derive(Default)]
pub(crate) struct ModelValidationCache {
    by_model: HashMap<String, bool>,
    warned_models: HashSet<String>,
}

#[derive(Default)]
pub(crate) struct ProxyAssetCache {
    mesh: Option<Handle<Mesh>>,
    materials: HashMap<u32, Handle<StandardMaterial>>,
}

/// System to spawn scene objects once assets are loaded
pub fn spawn_scene_objects_when_ready(
    mut commands: Commands,
    assets: Res<RuntimeSceneAssets>,
    scene_objects_data: Res<Assets<SceneObjectsData>>,
    particle_defs: Res<Assets<ParticleDefinitions>>,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut model_validation_cache: Local<ModelValidationCache>,
    mut proxy_assets: Local<ProxyAssetCache>,
    spawned_query: Query<&SceneObjectsSpawned>,
) {
    // Only spawn once
    if !spawned_query.is_empty() {
        return;
    }

    // Wait for assets to be loaded
    if !assets.loaded {
        return;
    }

    let Some(world) = assets.world.as_ref() else {
        return;
    };

    let Some(scene_data) = scene_objects_data.get(&world.scene_objects) else {
        return;
    };

    let Some(particle_definitions) = particle_defs.get(&assets.particle_defs) else {
        return;
    };

    let (object_defs, rotation_encoding) = if scene_data.objects.is_empty() {
        warn!(
            "Scene object list is empty; falling back to placeholder login objects. For parity with C++ scene, provide EncTerrain<world>.obj and regenerate scene_objects.json"
        );
        (
            fallback_scene_objects(),
            SceneRotationEncoding::LegacySwizzledDegrees,
        )
    } else {
        (
            scene_data.objects.clone(),
            scene_data.metadata.rotation_encoding,
        )
    };

    info!("Spawning {} scene objects", object_defs.len());
    let spawn_started_at = Instant::now();

    // Spawn each object
    for object in &object_defs {
        spawn_scene_object(
            &mut commands,
            &asset_server,
            &mut meshes,
            &mut materials,
            &mut model_validation_cache,
            &mut proxy_assets,
            object,
            particle_definitions,
            rotation_encoding,
        );
    }

    // Mark as spawned
    commands.spawn((SceneObjectsSpawned, RuntimeSceneEntity));

    info!(
        "Scene objects spawned successfully in {} ms",
        spawn_started_at.elapsed().as_millis()
    );
}

/// Spawn a single scene object
fn spawn_scene_object(
    commands: &mut Commands,
    asset_server: &AssetServer,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    model_validation_cache: &mut ModelValidationCache,
    proxy_assets: &mut ProxyAssetCache,
    object_def: &SceneObjectDef,
    particle_defs: &ParticleDefinitions,
    rotation_encoding: SceneRotationEncoding,
) {
    let position = Vec3::from(object_def.position);
    let rotation = scene_object_rotation_to_quat(object_def.rotation, rotation_encoding);
    let scale = Vec3::from(object_def.scale);

    let mut entity_cmd = commands.spawn((
        RuntimeSceneEntity,
        SceneObject,
        SpatialBundle {
            transform: Transform {
                translation: position,
                rotation,
                scale,
            },
            ..default()
        },
    ));

    if object_def.model.is_empty() {
        spawn_model_proxy(
            &mut entity_cmd,
            meshes,
            materials,
            proxy_assets,
            object_def.object_type,
        );
    } else if matches!(object_def.properties.model_renderable, Some(false)) {
        if model_validation_cache
            .warned_models
            .insert(object_def.model.clone())
        {
            let reason = object_def
                .properties
                .model_validation_reason
                .as_deref()
                .unwrap_or("precomputed conversion validation failed");
            warn!(
                "Model '{}' marked as non-renderable by conversion pipeline ({}). Using proxy mesh.",
                object_def.model, reason
            );
        }
        spawn_model_proxy(
            &mut entity_cmd,
            meshes,
            materials,
            proxy_assets,
            object_def.object_type,
        );
    } else if matches!(object_def.properties.model_renderable, Some(true))
        || is_renderable_model(&object_def.model, model_validation_cache)
    {
        let scene_path = normalize_scene_path(&object_def.model);
        let scene: Handle<Scene> = asset_server.load(scene_path);
        entity_cmd.with_children(|parent| {
            parent.spawn(SceneBundle { scene, ..default() });
        });
    } else {
        spawn_model_proxy(
            &mut entity_cmd,
            meshes,
            materials,
            proxy_assets,
            object_def.object_type,
        );
    }

    // Add particle emitter if specified
    if let Some(emitter_type) = &object_def.properties.particle_emitter {
        if let Some(emitter_def) = particle_defs.emitters.get(emitter_type) {
            add_particle_emitter(&mut entity_cmd, emitter_def);
        } else {
            warn!(
                "Particle emitter '{}' not found for object '{}'",
                emitter_type, object_def.id
            );
        }
    }

    // Add dynamic light if specified
    if let Some(light_color) = object_def.properties.light_color {
        add_dynamic_light(&mut entity_cmd, &object_def.properties, light_color);
    }

    // Add boid spawner if object type is 62 (eagle spawn point)
    if object_def.object_type == 62 {
        spawn_boid(commands, object_def);
    }
}

fn is_renderable_model(model_path: &str, cache: &mut ModelValidationCache) -> bool {
    if let Some(is_renderable) = cache.by_model.get(model_path) {
        return *is_renderable;
    }

    let validation = validate_model_asset(model_path);
    let is_renderable = validation.is_ok();
    if !is_renderable && cache.warned_models.insert(model_path.to_string()) {
        if let Err(reason) = &validation {
            warn!(
                "Model '{}' is not renderable ({}). Using proxy mesh.",
                model_path, reason
            );
        } else {
            warn!(
                "Model '{}' is not renderable. Using proxy mesh.",
                model_path
            );
        }
    }
    cache.by_model.insert(model_path.to_string(), is_renderable);
    is_renderable
}

fn validate_model_asset(model_path: &str) -> Result<(), String> {
    let normalized_model_path = model_path.split('#').next().unwrap_or(model_path);
    let full_path = asset_disk_path(normalized_model_path);
    if !full_path.exists() {
        return Err(format!("asset path not found: {}", full_path.display()));
    }

    match full_path
        .extension()
        .and_then(|extension| extension.to_str())
    {
        Some(ext) if ext.eq_ignore_ascii_case("glb") => validate_glb_asset(&full_path),
        Some(ext) if ext.eq_ignore_ascii_case("gltf") => {
            Err("gltf is no longer supported; use glb".to_string())
        }
        _ => Ok(()),
    }
}

fn asset_disk_path(asset_path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("assets")
        .join(asset_path)
}

fn validate_glb_asset(path: &Path) -> Result<(), String> {
    let size = fs::metadata(path)
        .map_err(|error| format!("failed to stat GLB '{}': {}", path.display(), error))?
        .len();
    if size < 128 {
        return Err(format!("GLB payload too small ({} bytes)", size));
    }
    Ok(())
}

fn spawn_model_proxy(
    entity_cmd: &mut EntityCommands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    proxy_assets: &mut ProxyAssetCache,
    object_type: u32,
) {
    let mesh_handle = proxy_assets
        .mesh
        .get_or_insert_with(|| meshes.add(Mesh::from(Cuboid::new(260.0, 420.0, 260.0))))
        .clone();
    let material_handle = proxy_assets
        .materials
        .entry(object_type)
        .or_insert_with(|| {
            let hue = (object_type % 360) as f32;
            materials.add(StandardMaterial {
                base_color: Color::hsl(hue, 0.75, 0.62),
                perceptual_roughness: 0.9,
                metallic: 0.0,
                unlit: true,
                ..default()
            })
        })
        .clone();

    entity_cmd.with_children(|parent| {
        parent.spawn(PbrBundle {
            mesh: mesh_handle,
            material: material_handle,
            transform: Transform::from_xyz(0.0, 210.0, 0.0),
            ..default()
        });
    });
}

/// Add particle emitter component to entity
fn add_particle_emitter(entity_cmd: &mut EntityCommands, emitter_def: &ParticleEmitterDef) {
    let config = ParticleEmitterConfig {
        lifetime_range: (emitter_def.lifetime[0], emitter_def.lifetime[1]),
        initial_velocity: Vec3::from(emitter_def.initial_velocity),
        velocity_variance: Vec3::from(emitter_def.velocity_variance),
    };

    entity_cmd.insert(ParticleEmitter {
        config,
        active: true,
        particles: Vec::new(),
        spawn_timer: Timer::from_seconds(1.0 / emitter_def.spawn_rate, TimerMode::Repeating),
    });
}

/// Add dynamic light component to entity
fn add_dynamic_light(
    entity_cmd: &mut EntityCommands,
    properties: &ObjectProperties,
    light_color: [f32; 3],
) {
    entity_cmd.insert(DynamicLight {
        color: Color::srgb(light_color[0], light_color[1], light_color[2]),
        intensity: properties.light_intensity.unwrap_or(1.0),
        range: properties.light_range.unwrap_or(5.0),
        flicker: Some(FlickerParams {
            min_intensity: 0.3,
            max_intensity: 0.7,
            speed: 2.0,
        }),
    });
}

/// Spawn a boid (eagle) at the object location
fn spawn_boid(commands: &mut Commands, object_def: &SceneObjectDef) {
    let spawn_point = Vec3::from(object_def.position);
    let flight_radius = object_def.properties.flight_radius.unwrap_or(30.0);

    commands.spawn((
        RuntimeSceneEntity,
        SpatialBundle {
            transform: Transform::from_translation(spawn_point),
            ..default()
        },
        Boid {
            spawn_point,
            animation_timer: Timer::from_seconds(0.1, TimerMode::Repeating),
        },
        BoidFlightPattern {
            pattern_type: FlightPattern::Circular {
                radius: flight_radius,
                speed: 0.3,
            },
            time: 0.0,
        },
    ));
}

fn fallback_scene_objects() -> Vec<SceneObjectDef> {
    vec![
        SceneObjectDef {
            id: "fallback_gate_1".to_string(),
            object_type: 113,
            model: "data/Object74/Object114.glb".to_string(),
            position: [24_000.0, 170.0, 2_600.0],
            rotation: [0.0, 125.0, 0.0],
            scale: [1.0, 1.0, 1.0],
            properties: ObjectProperties::default(),
        },
        SceneObjectDef {
            id: "fallback_tower_1".to_string(),
            object_type: 122,
            model: "data/Object74/Object123.glb".to_string(),
            position: [23_200.0, 170.0, 3_200.0],
            rotation: [0.0, 0.0, 0.0],
            scale: [1.0, 1.0, 1.0],
            properties: ObjectProperties::default(),
        },
        SceneObjectDef {
            id: "fallback_wall_1".to_string(),
            object_type: 126,
            model: "data/Object74/Object127.glb".to_string(),
            position: [22_100.0, 170.0, 4_300.0],
            rotation: [0.0, 180.0, 0.0],
            scale: [1.0, 1.0, 1.0],
            properties: ObjectProperties::default(),
        },
        SceneObjectDef {
            id: "fallback_hall_2".to_string(),
            object_type: 139,
            model: "data/Object74/Object140.glb".to_string(),
            position: [20_900.0, 170.0, 4_900.0],
            rotation: [0.0, 210.0, 0.0],
            scale: [1.0, 1.0, 1.0],
            properties: ObjectProperties::default(),
        },
        SceneObjectDef {
            id: "fallback_hall_3".to_string(),
            object_type: 145,
            model: "data/Object74/Object146.glb".to_string(),
            position: [19_900.0, 170.0, 5_100.0],
            rotation: [0.0, 235.0, 0.0],
            scale: [1.0, 1.0, 1.0],
            properties: ObjectProperties::default(),
        },
        SceneObjectDef {
            id: "fallback_wall_2".to_string(),
            object_type: 148,
            model: "data/Object74/Object149.glb".to_string(),
            position: [20_600.0, 170.0, 2_300.0],
            rotation: [0.0, 30.0, 0.0],
            scale: [1.0, 1.0, 1.0],
            properties: ObjectProperties::default(),
        },
        SceneObjectDef {
            id: "fallback_arch_1".to_string(),
            object_type: 70,
            model: "data/Object74/Object71.glb".to_string(),
            position: [23_100.0, 170.0, 1_900.0],
            rotation: [0.0, 95.0, 0.0],
            scale: [1.0, 1.0, 1.0],
            properties: ObjectProperties::default(),
        },
        SceneObjectDef {
            id: "fallback_fire_1".to_string(),
            object_type: 103,
            model: "data/Object74/Object104.glb".to_string(),
            position: [21_100.0, 170.0, 2_700.0],
            rotation: [0.0, 0.0, 0.0],
            scale: [1.0, 1.0, 1.0],
            properties: ObjectProperties {
                particle_emitter: Some("fire_orange".to_string()),
                light_color: Some([1.0, 0.6, 0.2]),
                light_intensity: Some(300.0),
                light_range: Some(350.0),
                ..Default::default()
            },
        },
        SceneObjectDef {
            id: "fallback_cloud_1".to_string(),
            object_type: 60,
            model: "data/Object74/Object63.glb".to_string(),
            position: [20_900.0, 260.0, 3_000.0],
            rotation: [0.0, 0.0, 0.0],
            scale: [0.7, 0.7, 0.7],
            properties: ObjectProperties {
                particle_emitter: Some("cloud".to_string()),
                ..Default::default()
            },
        },
        SceneObjectDef {
            id: "fallback_eagle_spawn".to_string(),
            object_type: 62,
            model: "data/Object74/Object63.glb".to_string(),
            position: [20_800.0, 300.0, 3_400.0],
            rotation: [0.0, 0.0, 0.0],
            scale: [1.0, 1.0, 1.0],
            properties: ObjectProperties {
                flight_radius: Some(900.0),
                flight_height: Some(250.0),
                ..Default::default()
            },
        },
    ]
}

fn normalize_scene_path(model_path: &str) -> String {
    if model_path.contains('#') {
        return model_path.to_string();
    }

    if model_path.ends_with(".glb") {
        format!("{model_path}#Scene0")
    } else {
        model_path.to_string()
    }
}
