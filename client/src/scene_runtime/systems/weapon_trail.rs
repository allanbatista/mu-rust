use crate::bevy_compat::*;
use crate::scene_runtime::components::*;
use bevy::image::{ImageAddressMode, ImageLoaderSettings, ImageSampler, ImageSamplerDescriptor};
use bevy::light::{NotShadowCaster, NotShadowReceiver};
use bevy::mesh::PrimitiveTopology;
use bevy::prelude::*;
use std::collections::HashMap;

/// Cached trail materials keyed by (texture_path, additive).
#[derive(Default)]
pub(crate) struct TrailMaterialCache {
    materials: HashMap<(String, bool), Handle<StandardMaterial>>,
}

/// Update all active weapon trails: age/cull samples, sample new positions,
/// ensure render entity, rebuild the ribbon mesh each frame.
pub fn update_weapon_trails(
    mut commands: Commands,
    time: Res<Time>,
    mut trails: Query<(Entity, &mut WeaponTrail)>,
    global_transforms: Query<&GlobalTransform>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    mut material_cache: Local<TrailMaterialCache>,
) {
    let dt = time.delta_secs();

    for (trail_entity, mut trail) in &mut trails {
        trail.elapsed += dt;

        // Age and cull expired samples
        for sample in trail.samples.iter_mut() {
            sample.age += dt;
        }
        while trail
            .samples
            .front()
            .is_some_and(|s| s.age >= trail.config.sample_lifetime)
        {
            trail.samples.pop_front();
        }

        trail.time_since_last_sample += dt;

        // Sample new positions while still within active duration
        if trail.elapsed <= trail.active_duration {
            let bone_result = global_transforms
                .get(trail.config.hand_bone)
                .ok()
                .zip(global_transforms.get(trail.config.tip_bone).ok());

            if let Some((hand_gt, tip_gt)) = bone_result {
                let hand_pos = hand_gt.translation();
                let tip_pos = tip_gt.translation();
                let mut blade_dir = tip_pos - hand_pos;
                if blade_dir.length_squared() <= f32::EPSILON {
                    blade_dir = hand_gt.rotation().mul_vec3(Vec3::NEG_Y);
                }
                if blade_dir.length_squared() <= f32::EPSILON {
                    blade_dir = Vec3::NEG_Z;
                }
                blade_dir = blade_dir.normalize();

                let near = hand_pos + blade_dir * trail.config.near_offset;
                let far = hand_pos + blade_dir * trail.config.far_offset;
                push_trail_sample(&mut trail, near, far);
            }
        }

        // Despawn entire trail when done and all samples expired
        if trail.elapsed > trail.active_duration + trail.config.sample_lifetime
            && trail.samples.is_empty()
        {
            if let Some(mesh_entity) = trail.mesh_entity {
                commands.entity(mesh_entity).despawn();
            }
            commands.entity(trail_entity).despawn();
            continue;
        }

        // Ensure render entity exists
        if trail.mesh_entity.is_none() {
            let cache_key = (trail.config.texture_path.clone(), trail.config.additive);
            let material_handle =
                material_cache
                    .materials
                    .entry(cache_key.clone())
                    .or_insert_with(|| {
                        let texture = asset_server.load_with_settings(
                            cache_key.0.clone(),
                            |settings: &mut _| {
                                *settings = ImageLoaderSettings {
                                    is_srgb: true,
                                    sampler: ImageSampler::Descriptor(ImageSamplerDescriptor {
                                        address_mode_u: ImageAddressMode::ClampToEdge,
                                        address_mode_v: ImageAddressMode::ClampToEdge,
                                        ..default()
                                    }),
                                    ..default()
                                };
                            },
                        );
                        materials.add(StandardMaterial {
                            base_color_texture: Some(texture),
                            base_color: Color::WHITE,
                            alpha_mode: if cache_key.1 {
                                AlphaMode::Add
                            } else {
                                AlphaMode::Blend
                            },
                            unlit: true,
                            double_sided: true,
                            cull_mode: None,
                            perceptual_roughness: 1.0,
                            metallic: 0.0,
                            reflectance: 0.0,
                            ..default()
                        })
                    });

            let mesh_handle = meshes.add(empty_trail_mesh());
            trail.mesh_handle = Some(mesh_handle.clone());
            let mesh_entity = commands
                .spawn((
                    NotShadowCaster,
                    NotShadowReceiver,
                    PbrBundle {
                        mesh: Mesh3d(mesh_handle),
                        material: MeshMaterial3d(material_handle.clone()),
                        transform: Transform::IDENTITY,
                        ..default()
                    },
                ))
                .id();
            trail.mesh_entity = Some(mesh_entity);
        }

        // Rebuild mesh from samples
        if let Some(ref mesh_handle) = trail.mesh_handle {
            if let Some(mesh) = meshes.get_mut(mesh_handle) {
                *mesh = build_trail_mesh(&trail);
            }
        }
    }
}

fn push_trail_sample(trail: &mut WeaponTrail, near: Vec3, far: Vec3) {
    let should_sample = match trail.samples.back() {
        Some(last) => {
            let movement = last
                .near
                .distance_squared(near)
                .max(last.far.distance_squared(far));
            movement >= trail.config.min_sample_distance_sq
                || trail.time_since_last_sample >= trail.config.max_sample_interval
        }
        None => true,
    };

    if !should_sample {
        return;
    }

    if trail.samples.len() >= trail.config.max_samples {
        trail.samples.pop_front();
    }
    trail.samples.push_back(WeaponTrailSample {
        near,
        far,
        age: 0.0,
    });
    trail.time_since_last_sample = 0.0;
}

fn build_trail_mesh(trail: &WeaponTrail) -> Mesh {
    let sample_count = trail.samples.len();
    if sample_count < 2 {
        return empty_trail_mesh();
    }

    let quad_count = sample_count - 1;
    let vertex_count = quad_count * 6;
    let mut positions: Vec<[f32; 3]> = Vec::with_capacity(vertex_count);
    let mut normals: Vec<[f32; 3]> = Vec::with_capacity(vertex_count);
    let mut uvs: Vec<[f32; 2]> = Vec::with_capacity(vertex_count);
    let mut colors: Vec<[f32; 4]> = Vec::with_capacity(vertex_count);

    let samples: Vec<&WeaponTrailSample> = trail.samples.iter().collect();

    for i in 0..quad_count {
        let s0 = samples[i];
        let s1 = samples[i + 1];

        // UV: U along ribbon length (0=oldest, 1=newest), V across width (0=near, 1=far)
        let u0 = i as f32 / (sample_count - 1) as f32;
        let u1 = (i + 1) as f32 / (sample_count - 1) as f32;

        // Life factor: 1.0 when fresh, 0.0 when expired
        let life0 = (1.0 - s0.age / trail.config.sample_lifetime).clamp(0.0, 1.0);
        let life1 = (1.0 - s1.age / trail.config.sample_lifetime).clamp(0.0, 1.0);

        // Lerp color based on life (new=color_new at life 1.0, old=color_old at life 0.0)
        let c0 = lerp_color(trail.config.color_old, trail.config.color_new, life0);
        let c1 = lerp_color(trail.config.color_old, trail.config.color_new, life1);

        // Face normal from cross product
        let edge_along = s1.near - s0.near;
        let edge_across = s0.far - s0.near;
        let face_normal = edge_along.cross(edge_across).normalize_or_zero();
        let n: [f32; 3] = face_normal.into();

        let p0_near: [f32; 3] = s0.near.into();
        let p0_far: [f32; 3] = s0.far.into();
        let p1_near: [f32; 3] = s1.near.into();
        let p1_far: [f32; 3] = s1.far.into();

        // Triangle 1: s0.near, s1.near, s1.far
        positions.extend_from_slice(&[p0_near, p1_near, p1_far]);
        normals.extend_from_slice(&[n, n, n]);
        uvs.extend_from_slice(&[[u0, 0.0], [u1, 0.0], [u1, 1.0]]);
        colors.extend_from_slice(&[c0, c1, c1]);

        // Triangle 2: s0.near, s1.far, s0.far
        positions.extend_from_slice(&[p0_near, p1_far, p0_far]);
        normals.extend_from_slice(&[n, n, n]);
        uvs.extend_from_slice(&[[u0, 0.0], [u1, 1.0], [u0, 1.0]]);
        colors.extend_from_slice(&[c0, c1, c0]);
    }

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh
}

fn lerp_color(a: [f32; 4], b: [f32; 4], t: f32) -> [f32; 4] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
        a[3] + (b[3] - a[3]) * t,
    ]
}

fn empty_trail_mesh() -> Mesh {
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, Vec::<[f32; 3]>::new());
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, Vec::<[f32; 3]>::new());
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, Vec::<[f32; 2]>::new());
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, Vec::<[f32; 4]>::new());
    mesh
}
