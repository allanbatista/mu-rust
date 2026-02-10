use crate::bevy_compat::*;
use crate::scene_runtime::components::*;
use bevy::image::{ImageAddressMode, ImageLoaderSettings, ImageSampler, ImageSamplerDescriptor};
use bevy::light::{NotShadowCaster, NotShadowReceiver};
use bevy::mesh::PrimitiveTopology;
use bevy::prelude::*;
use rand::Rng;
use std::collections::{HashMap, HashSet};
use std::f32::consts::TAU;

const MAX_PARTICLE_SPAWNS_PER_FRAME: u32 = 32;
const MAX_RENDERED_PARTICLES_PER_FRAME: usize = 2048;
const MAX_RENDER_DISTANCE_SQUARED: f32 = 3_000.0 * 3_000.0;
const MAX_SIMULATION_DISTANCE_SQUARED: f32 = 2_200.0 * 2_200.0;
const FAR_FIELD_PARTICLE_DECAY_MULTIPLIER: f32 = 3.5;
const PARTICLE_GRAVITY: f32 = 9.8;

/// Render batch keyed by particle texture + blend mode.
#[derive(Component, Clone, Debug, Eq, PartialEq, Hash)]
pub struct ParticleRenderBatch {
    pub texture_path: String,
    pub blend_mode: ParticleBlendMode,
}

/// Build a runtime particle emitter from JSON definitions.
pub(super) fn particle_emitter_from_definition(
    emitter_def: &ParticleEmitterDef,
) -> Option<ParticleEmitter> {
    let spawn_rate = if emitter_def.spawn_rate.is_finite() {
        emitter_def.spawn_rate
    } else {
        0.0
    }
    .max(0.001);

    let lifetime_min = if emitter_def.lifetime[0].is_finite() {
        emitter_def.lifetime[0]
    } else {
        0.1
    }
    .max(0.05);
    let lifetime_max = if emitter_def.lifetime[1].is_finite() {
        emitter_def.lifetime[1]
    } else {
        lifetime_min
    }
    .max(lifetime_min);

    let scale_min = if emitter_def.scale[0].is_finite() {
        emitter_def.scale[0]
    } else {
        0.25
    }
    .max(0.01);
    let scale_max = if emitter_def.scale[1].is_finite() {
        emitter_def.scale[1]
    } else {
        scale_min
    }
    .max(scale_min);

    let max_particles = ((spawn_rate * lifetime_max * 1.7).ceil() as usize).clamp(16, 512);
    let texture_path = emitter_def.texture.trim().replace('\\', "/");
    if texture_path.is_empty() {
        return None;
    }

    let scale_variance = if emitter_def.scale_variance.is_finite() {
        emitter_def.scale_variance
    } else {
        0.0
    }
    .max(0.0);
    let rotation_speed = emitter_def
        .rotation_speed
        .and_then(|speed| speed.is_finite().then_some(speed))
        .unwrap_or(0.0);

    let config = ParticleEmitterConfig {
        lifetime_range: (lifetime_min, lifetime_max),
        initial_velocity: Vec3::from(emitter_def.initial_velocity),
        velocity_variance: Vec3::from(emitter_def.velocity_variance),
        scale_range: (scale_min, scale_max),
        scale_variance,
        color_start: Vec4::from_array(emitter_def.color),
        color_end: Vec4::from_array(emitter_def.color_fade),
        texture_path,
        blend_mode: parse_particle_blend_mode(&emitter_def.blend_mode),
        rotation_speed,
        max_particles,
    };

    Some(ParticleEmitter {
        config,
        active: true,
        particles: Vec::new(),
        spawn_timer: Timer::from_seconds(1.0 / spawn_rate, TimerMode::Repeating),
    })
}

fn parse_particle_blend_mode(raw_mode: &str) -> ParticleBlendMode {
    match raw_mode.trim().to_ascii_lowercase().as_str() {
        "add" | "additive" => ParticleBlendMode::Additive,
        _ => ParticleBlendMode::Alpha,
    }
}

/// Ensure one persistent render entity per particle batch key.
pub fn ensure_particle_render_batches(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    emitters: Query<&ParticleEmitter, With<RuntimeSceneEntity>>,
    batches: Query<(Entity, &ParticleRenderBatch)>,
) {
    let desired_batches: HashSet<ParticleRenderBatch> = emitters
        .iter()
        .map(|emitter| ParticleRenderBatch {
            texture_path: emitter.config.texture_path.clone(),
            blend_mode: emitter.config.blend_mode,
        })
        .collect();

    let mut existing_batches: HashMap<ParticleRenderBatch, Entity> = HashMap::new();
    for (entity, batch) in &batches {
        existing_batches.insert(batch.clone(), entity);
    }

    for (batch, entity) in &existing_batches {
        if !desired_batches.contains(batch) {
            commands.entity(*entity).despawn();
        }
    }

    for batch in desired_batches {
        if existing_batches.contains_key(&batch) {
            continue;
        }

        let texture =
            asset_server.load_with_settings(batch.texture_path.clone(), |settings: &mut _| {
                *settings = ImageLoaderSettings {
                    is_srgb: true,
                    sampler: ImageSampler::Descriptor(ImageSamplerDescriptor {
                        address_mode_u: ImageAddressMode::ClampToEdge,
                        address_mode_v: ImageAddressMode::ClampToEdge,
                        ..default()
                    }),
                    ..default()
                };
            });

        let material_handle = materials.add(StandardMaterial {
            base_color_texture: Some(texture),
            base_color: Color::WHITE,
            alpha_mode: match batch.blend_mode {
                ParticleBlendMode::Alpha => AlphaMode::Blend,
                ParticleBlendMode::Additive => AlphaMode::Add,
            },
            unlit: true,
            double_sided: true,
            cull_mode: None,
            perceptual_roughness: 1.0,
            metallic: 0.0,
            reflectance: 0.0,
            ..default()
        });

        let mesh_handle = meshes.add(empty_particle_mesh());
        commands.spawn((
            RuntimeSceneEntity,
            batch,
            NotShadowCaster,
            NotShadowReceiver,
            PbrBundle {
                mesh: Mesh3d(mesh_handle),
                material: MeshMaterial3d(material_handle),
                transform: Transform::IDENTITY,
                visibility: Visibility::Hidden,
                ..default()
            },
        ));
    }
}

/// Update particle emitters with distance-based simulation culling.
pub fn update_particle_emitters(
    mut emitters: Query<(&mut ParticleEmitter, &GlobalTransform)>,
    camera_query: Query<&GlobalTransform, With<Camera3d>>,
    time: Res<Time>,
) {
    let camera_position = camera_query.single().ok().map(GlobalTransform::translation);
    let delta_seconds = time.delta_secs();

    for (mut emitter, transform) in &mut emitters {
        if !emitter.active {
            continue;
        }

        let max_particles = emitter.config.max_particles.max(1);
        if emitter.particles.len() > max_particles {
            emitter.particles.truncate(max_particles);
        }

        if let Some(camera_position) = camera_position {
            if transform.translation().distance_squared(camera_position)
                > MAX_SIMULATION_DISTANCE_SQUARED
            {
                emitter.spawn_timer.reset();
                if !emitter.particles.is_empty() {
                    decay_particles(
                        &mut emitter.particles,
                        delta_seconds * FAR_FIELD_PARTICLE_DECAY_MULTIPLIER,
                    );
                }
                continue;
            }
        }

        emitter.spawn_timer.tick(time.delta());
        let spawn_count = emitter
            .spawn_timer
            .times_finished_this_tick()
            .min(MAX_PARTICLE_SPAWNS_PER_FRAME);
        for _ in 0..spawn_count {
            if emitter.particles.len() >= max_particles {
                break;
            }
            spawn_particle(&mut emitter, transform.translation());
        }

        update_particles(&mut emitter.particles, delta_seconds);
    }
}

/// Update render batches with one mesh per particle batch key.
pub fn update_particle_render_batches(
    emitters: Query<&ParticleEmitter>,
    mut batches: Query<(&ParticleRenderBatch, &Mesh3d, &mut Visibility)>,
    camera_query: Query<&GlobalTransform, With<Camera3d>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let Ok(camera_transform) = camera_query.single() else {
        return;
    };
    let (_, camera_rotation, camera_position) = camera_transform.to_scale_rotation_translation();
    let camera_right = camera_rotation * Vec3::X;
    let camera_up = camera_rotation * Vec3::Y;

    let mut remaining_budget = MAX_RENDERED_PARTICLES_PER_FRAME;

    for (batch, mesh_handle, mut visibility) in &mut batches {
        let Some(mesh) = meshes.get_mut(&mesh_handle.0) else {
            continue;
        };

        if remaining_budget == 0 {
            *visibility = Visibility::Hidden;
            *mesh = empty_particle_mesh();
            continue;
        }

        let (built, rendered_particles) = build_particle_billboard_mesh_for_batch(
            &emitters,
            batch,
            remaining_budget,
            camera_position,
            camera_right,
            camera_up,
        );

        if rendered_particles == 0 {
            *visibility = Visibility::Hidden;
            *mesh = empty_particle_mesh();
            continue;
        }

        *visibility = Visibility::Inherited;
        *mesh = built;
        remaining_budget = remaining_budget.saturating_sub(rendered_particles);
    }
}

fn spawn_particle(emitter: &mut ParticleEmitter, position: Vec3) {
    let mut rng = rand::thread_rng();

    let lifetime = rng.gen_range(emitter.config.lifetime_range.0..=emitter.config.lifetime_range.1);
    let velocity_offset = Vec3::new(
        rng.gen_range(-emitter.config.velocity_variance.x..=emitter.config.velocity_variance.x),
        rng.gen_range(-emitter.config.velocity_variance.y..=emitter.config.velocity_variance.y),
        rng.gen_range(-emitter.config.velocity_variance.z..=emitter.config.velocity_variance.z),
    );
    let base_scale = rng.gen_range(emitter.config.scale_range.0..=emitter.config.scale_range.1);
    let scale_jitter = if emitter.config.scale_variance > 0.0 {
        rng.gen_range(-emitter.config.scale_variance..=emitter.config.scale_variance)
    } else {
        0.0
    };
    let scale = (base_scale + scale_jitter).max(0.01);
    let rotation = rng.gen_range(0.0..TAU);
    let rotation_speed_jitter = rng.gen_range(0.75..=1.25);

    emitter.particles.push(Particle {
        position,
        velocity: emitter.config.initial_velocity + velocity_offset,
        lifetime: 0.0,
        max_lifetime: lifetime,
        scale,
        rotation,
        rotation_speed: emitter.config.rotation_speed * rotation_speed_jitter,
    });
}

fn decay_particles(particles: &mut Vec<Particle>, delta: f32) {
    particles.retain_mut(|particle| {
        particle.lifetime += delta;
        particle.lifetime < particle.max_lifetime
    });
}

fn update_particles(particles: &mut Vec<Particle>, delta: f32) {
    particles.retain_mut(|particle| {
        particle.lifetime += delta;
        if particle.lifetime >= particle.max_lifetime {
            return false;
        }

        particle.position += particle.velocity * delta;
        particle.velocity.y -= PARTICLE_GRAVITY * delta;
        particle.rotation += particle.rotation_speed * delta;
        true
    });
}

fn build_particle_billboard_mesh_for_batch(
    emitters: &Query<&ParticleEmitter>,
    batch: &ParticleRenderBatch,
    max_particles: usize,
    camera_position: Vec3,
    camera_right: Vec3,
    camera_up: Vec3,
) -> (Mesh, usize) {
    let max_vertices = max_particles * 6;
    let mut positions: Vec<[f32; 3]> = Vec::with_capacity(max_vertices);
    let mut normals: Vec<[f32; 3]> = Vec::with_capacity(max_vertices);
    let mut uvs: Vec<[f32; 2]> = Vec::with_capacity(max_vertices);
    let mut colors: Vec<[f32; 4]> = Vec::with_capacity(max_vertices);

    let mut emitted = 0usize;

    'emitters: for emitter in emitters.iter() {
        if emitter.config.blend_mode != batch.blend_mode
            || emitter.config.texture_path != batch.texture_path
        {
            continue;
        }

        for particle in &emitter.particles {
            if emitted >= max_particles {
                break 'emitters;
            }
            if particle.position.distance_squared(camera_position) > MAX_RENDER_DISTANCE_SQUARED {
                continue;
            }

            let age = (particle.lifetime / particle.max_lifetime).clamp(0.0, 1.0);
            let color = emitter
                .config
                .color_start
                .lerp(emitter.config.color_end, age)
                .max(Vec4::ZERO);
            if color.w <= 0.01 {
                continue;
            }

            let size = (particle.scale * (1.0 - age * 0.35)).max(0.01);
            push_particle_quad(
                &mut positions,
                &mut normals,
                &mut uvs,
                &mut colors,
                particle.position,
                size,
                particle.rotation,
                camera_right,
                camera_up,
                color,
            );
            emitted += 1;
        }
    }

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    (mesh, emitted)
}

#[allow(clippy::too_many_arguments)]
fn push_particle_quad(
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    uvs: &mut Vec<[f32; 2]>,
    colors: &mut Vec<[f32; 4]>,
    center: Vec3,
    size: f32,
    rotation: f32,
    camera_right: Vec3,
    camera_up: Vec3,
    color: Vec4,
) {
    let half = size * 0.5;
    let corners = [
        Vec2::new(-half, -half),
        Vec2::new(half, -half),
        Vec2::new(half, half),
        Vec2::new(-half, half),
    ];
    let sin_rot = rotation.sin();
    let cos_rot = rotation.cos();

    let rotated_corner = |corner: Vec2| -> Vec2 {
        Vec2::new(
            corner.x * cos_rot - corner.y * sin_rot,
            corner.x * sin_rot + corner.y * cos_rot,
        )
    };

    let world = |corner: Vec2| -> Vec3 { center + camera_right * corner.x + camera_up * corner.y };

    let bl = world(rotated_corner(corners[0]));
    let br = world(rotated_corner(corners[1]));
    let tr = world(rotated_corner(corners[2]));
    let tl = world(rotated_corner(corners[3]));

    let normal = [0.0, 1.0, 0.0];
    let color = [color.x, color.y, color.z, color.w];

    // Triangle 1
    positions.extend_from_slice(&[bl.into(), br.into(), tr.into()]);
    normals.extend_from_slice(&[normal, normal, normal]);
    uvs.extend_from_slice(&[[0.0, 1.0], [1.0, 1.0], [1.0, 0.0]]);
    colors.extend_from_slice(&[color, color, color]);

    // Triangle 2
    positions.extend_from_slice(&[bl.into(), tr.into(), tl.into()]);
    normals.extend_from_slice(&[normal, normal, normal]);
    uvs.extend_from_slice(&[[0.0, 1.0], [1.0, 0.0], [0.0, 0.0]]);
    colors.extend_from_slice(&[color, color, color]);
}

fn empty_particle_mesh() -> Mesh {
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, Vec::<[f32; 3]>::new());
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, Vec::<[f32; 3]>::new());
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, Vec::<[f32; 2]>::new());
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, Vec::<[f32; 4]>::new());
    mesh
}
