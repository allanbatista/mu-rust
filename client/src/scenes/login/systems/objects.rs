use crate::scenes::login::components::*;
use crate::scenes::login::LoginSceneAssets;
use bevy::ecs::system::EntityCommands;
use bevy::prelude::*;

/// Marker component to track if scene objects have been spawned
#[derive(Component)]
pub struct SceneObjectsSpawned;

/// System to spawn scene objects once assets are loaded
pub fn spawn_scene_objects_when_ready(
    mut commands: Commands,
    assets: Res<LoginSceneAssets>,
    scene_objects_data: Res<Assets<SceneObjectsData>>,
    particle_defs: Res<Assets<ParticleDefinitions>>,
    asset_server: Res<AssetServer>,
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

    let Some(scene_data) = scene_objects_data.get(&assets.scene_objects) else {
        return;
    };

    let Some(particle_definitions) = particle_defs.get(&assets.particle_defs) else {
        return;
    };

    info!("Spawning {} scene objects", scene_data.objects.len());

    // Spawn each object
    for object in &scene_data.objects {
        spawn_scene_object(
            &mut commands,
            &asset_server,
            object,
            particle_definitions,
        );
    }

    // Mark as spawned
    commands.spawn(SceneObjectsSpawned);

    info!("Scene objects spawned successfully");
}

/// Spawn a single scene object
fn spawn_scene_object(
    commands: &mut Commands,
    asset_server: &AssetServer,
    object_def: &SceneObjectDef,
    particle_defs: &ParticleDefinitions,
) {
    let position = Vec3::from(object_def.position);
    let rotation = Quat::from_euler(
        EulerRot::XYZ,
        object_def.rotation[0].to_radians(),
        object_def.rotation[1].to_radians(),
        object_def.rotation[2].to_radians(),
    );
    let scale = Vec3::from(object_def.scale);

    let mut entity_cmd = commands.spawn((
        SceneObject {
            id: object_def.id.clone(),
            object_type: object_def.object_type,
        },
        SpatialBundle {
            transform: Transform {
                translation: position,
                rotation,
                scale,
            },
            ..default()
        },
    ));

    // Add particle emitter if specified
    if let Some(emitter_type) = &object_def.properties.particle_emitter {
        if let Some(emitter_def) = particle_defs.emitters.get(emitter_type) {
            add_particle_emitter(&mut entity_cmd, asset_server, emitter_def);
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
        spawn_boid(commands, asset_server, object_def);
    }
}

/// Add particle emitter component to entity
fn add_particle_emitter(
    entity_cmd: &mut EntityCommands,
    asset_server: &AssetServer,
    emitter_def: &ParticleEmitterDef,
) {
    let texture = asset_server.load(&emitter_def.texture);

    let blend_mode = match emitter_def.blend_mode.as_str() {
        "additive" => ParticleBlendMode::Additive,
        "alpha" => ParticleBlendMode::Alpha,
        _ => {
            warn!("Unknown blend mode: {}", emitter_def.blend_mode);
            ParticleBlendMode::Alpha
        }
    };

    let config = ParticleEmitterConfig {
        texture,
        spawn_rate: emitter_def.spawn_rate,
        lifetime_range: (emitter_def.lifetime[0], emitter_def.lifetime[1]),
        initial_velocity: Vec3::from(emitter_def.initial_velocity),
        velocity_variance: Vec3::from(emitter_def.velocity_variance),
        scale_range: (emitter_def.scale[0], emitter_def.scale[1]),
        scale_variance: emitter_def.scale_variance,
        color_start: Color::srgba(
            emitter_def.color[0],
            emitter_def.color[1],
            emitter_def.color[2],
            emitter_def.color[3],
        ),
        color_end: Color::srgba(
            emitter_def.color_fade[0],
            emitter_def.color_fade[1],
            emitter_def.color_fade[2],
            emitter_def.color_fade[3],
        ),
        blend_mode,
        rotation_speed: emitter_def.rotation_speed,
    };

    entity_cmd.insert(ParticleEmitter {
        config,
        active: true,
        particles: Vec::new(),
        spawn_timer: Timer::from_seconds(
            1.0 / emitter_def.spawn_rate,
            TimerMode::Repeating,
        ),
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
fn spawn_boid(commands: &mut Commands, asset_server: &AssetServer, object_def: &SceneObjectDef) {
    let model_path = object_def
        .properties
        .boid_model
        .as_ref()
        .unwrap_or(&"data/creatures/eagle.glb".to_string());

    let spawn_point = Vec3::from(object_def.position);
    let flight_radius = object_def.properties.flight_radius.unwrap_or(30.0);
    let flight_height = object_def.properties.flight_height.unwrap_or(50.0);

    commands.spawn((
        SpatialBundle {
            transform: Transform::from_translation(spawn_point),
            ..default()
        },
        Boid {
            boid_type: BoidType::Eagle,
            velocity: Vec3::ZERO,
            flight_radius,
            flight_height,
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
