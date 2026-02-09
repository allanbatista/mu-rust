use super::particles::particle_emitter_from_definition;
use crate::scene_runtime::components::*;
use crate::scene_runtime::state::RuntimeSceneAssets;
use bevy::math::primitives::Rectangle;
use bevy::pbr::{NotShadowCaster, NotShadowReceiver};
use bevy::prelude::*;
use bevy::render::texture::{
    ImageAddressMode, ImageLoaderSettings, ImageSampler, ImageSamplerDescriptor,
};
use std::collections::{HashMap, HashSet};
use std::f32::consts::TAU;
use std::path::Path;

const DEFAULT_LIGHT_INTENSITY: f32 = 300.0;
const DEFAULT_LIGHT_RANGE: f32 = 320.0;
const DEFAULT_SPRITE_MAX_DISTANCE: f32 = 2800.0;

#[derive(Component)]
pub struct MapVfxObjectApplied;

#[derive(Component)]
pub struct MapVfxBillboardSprite {
    pub anchor: Entity,
    pub vertical_offset: f32,
    pub base_size: f32,
    pub spin_speed: f32,
    pub pulse_amplitude: f32,
    pub pulse_speed: f32,
    pub phase: f32,
    pub max_distance_squared: f32,
}

#[derive(Default)]
pub(crate) struct SpriteAssetCache {
    mesh: Option<Handle<Mesh>>,
    materials: HashMap<SpriteMaterialKey, Handle<StandardMaterial>>,
}

#[derive(Clone, Hash, Eq, PartialEq)]
struct SpriteMaterialKey {
    texture_path: String,
    blend_mode: MapVfxBlendMode,
    rgba_8: [u8; 4],
}

#[derive(Default, Clone, Copy)]
struct RuleBudgetState {
    seen: u32,
    applied: u32,
}

/// Apply per-map VFX profile to scene objects once they are spawned.
pub fn apply_map_vfx_profile_to_scene_objects(
    mut commands: Commands,
    assets: Res<RuntimeSceneAssets>,
    particle_defs: Res<Assets<ParticleDefinitions>>,
    map_vfx_profiles: Res<Assets<MapVfxProfile>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    mut sprite_assets: Local<SpriteAssetCache>,
    mut warned_missing_emitters: Local<HashSet<String>>,
    objects: Query<(Entity, &SceneObjectKind), (With<SceneObject>, Without<MapVfxObjectApplied>)>,
    existing_emitters: Query<(), With<ParticleEmitter>>,
    existing_lights: Query<(), With<DynamicLight>>,
) {
    if !assets.loaded {
        return;
    }

    let Some(world) = assets.world.as_ref() else {
        return;
    };
    let Some(profile_handle) = world.map_vfx.as_ref() else {
        return;
    };
    let Some(profile) = map_vfx_profiles.get(profile_handle) else {
        return;
    };
    let Some(particle_definitions) = particle_defs.get(&assets.particle_defs) else {
        return;
    };

    if profile.object_overrides.is_empty() && profile.object_sprites.is_empty() {
        for (entity, _) in &objects {
            commands.entity(entity).insert(MapVfxObjectApplied);
        }
        return;
    }

    let mut overrides_by_type: HashMap<u32, Vec<usize>> = HashMap::new();
    for (index, rule) in profile.object_overrides.iter().enumerate() {
        overrides_by_type
            .entry(rule.object_type)
            .or_default()
            .push(index);
    }

    let mut sprites_by_type: HashMap<u32, Vec<usize>> = HashMap::new();
    for (index, rule) in profile.object_sprites.iter().enumerate() {
        sprites_by_type
            .entry(rule.object_type)
            .or_default()
            .push(index);
    }

    let mut override_budget = vec![RuleBudgetState::default(); profile.object_overrides.len()];
    let mut sprite_budget = vec![RuleBudgetState::default(); profile.object_sprites.len()];
    let quad_mesh = sprite_assets
        .mesh
        .get_or_insert_with(|| meshes.add(Mesh::from(Rectangle::new(1.0, 1.0))))
        .clone();

    let mut applied_overrides = 0usize;
    let mut spawned_sprites = 0usize;

    for (entity, kind) in &objects {
        if let Some(rule_indices) = overrides_by_type.get(&kind.0) {
            for rule_index in rule_indices {
                let rule = &profile.object_overrides[*rule_index];
                if !consume_rule_budget(
                    rule.spawn_stride,
                    rule.max_instances,
                    &mut override_budget[*rule_index],
                ) {
                    continue;
                }

                if let Some(emitter_name) = &rule.particle_emitter {
                    if !existing_emitters.contains(entity) {
                        if let Some(definition) = particle_definitions.emitters.get(emitter_name) {
                            if let Some(emitter) = particle_emitter_from_definition(definition) {
                                commands.entity(entity).insert(emitter);
                            }
                        } else if warned_missing_emitters.insert(emitter_name.clone()) {
                            warn!(
                                "Map VFX profile references unknown particle emitter '{}'",
                                emitter_name
                            );
                        }
                    }
                }

                if let Some(color) = rule.light_color {
                    if !existing_lights.contains(entity) {
                        let flicker = rule.flicker.as_ref().map(|flicker| FlickerParams {
                            min_intensity: flicker.min_intensity.max(0.0),
                            max_intensity: flicker.max_intensity.max(0.0),
                            speed: flicker.speed.max(0.01),
                        });
                        commands.entity(entity).insert(DynamicLight {
                            color: Color::srgb(
                                color[0].max(0.0),
                                color[1].max(0.0),
                                color[2].max(0.0),
                            ),
                            intensity: rule
                                .light_intensity
                                .filter(|value| value.is_finite() && *value > 0.0)
                                .unwrap_or(DEFAULT_LIGHT_INTENSITY),
                            range: rule
                                .light_range
                                .filter(|value| value.is_finite() && *value > 0.0)
                                .unwrap_or(DEFAULT_LIGHT_RANGE),
                            flicker,
                        });
                    }
                }

                applied_overrides += 1;
                break;
            }
        }

        if let Some(rule_indices) = sprites_by_type.get(&kind.0) {
            for rule_index in rule_indices {
                let rule = &profile.object_sprites[*rule_index];
                if !consume_rule_budget(
                    rule.spawn_stride,
                    rule.max_instances,
                    &mut sprite_budget[*rule_index],
                ) {
                    continue;
                }

                let Some(texture_path) = normalize_existing_asset_path(&rule.texture) else {
                    warn!(
                        "Map VFX sprite texture '{}' could not be resolved",
                        rule.texture
                    );
                    continue;
                };

                let color = sanitize_color(rule.color);
                let key = SpriteMaterialKey {
                    texture_path: texture_path.clone(),
                    blend_mode: rule.blend_mode,
                    rgba_8: quantize_color(color),
                };

                let material_handle = if let Some(existing) = sprite_assets.materials.get(&key) {
                    existing.clone()
                } else {
                    let texture = asset_server.load_with_settings(
                        texture_path.clone(),
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

                    let created = materials.add(StandardMaterial {
                        base_color_texture: Some(texture),
                        base_color: Color::srgba(color[0], color[1], color[2], color[3]),
                        alpha_mode: match rule.blend_mode {
                            MapVfxBlendMode::Alpha => AlphaMode::Blend,
                            MapVfxBlendMode::Additive => AlphaMode::Add,
                        },
                        unlit: true,
                        double_sided: true,
                        cull_mode: None,
                        perceptual_roughness: 1.0,
                        metallic: 0.0,
                        reflectance: 0.0,
                        ..default()
                    });
                    sprite_assets.materials.insert(key, created.clone());
                    created
                };

                let max_distance = rule
                    .max_distance
                    .filter(|value| value.is_finite() && *value > 0.0)
                    .unwrap_or(DEFAULT_SPRITE_MAX_DISTANCE);
                let phase = phase_from_entity(entity, *rule_index as u32);

                commands.spawn((
                    RuntimeSceneEntity,
                    MapVfxBillboardSprite {
                        anchor: entity,
                        vertical_offset: rule.z_offset,
                        base_size: rule.size.max(1.0),
                        spin_speed: rule.spin_speed,
                        pulse_amplitude: rule
                            .pulse
                            .as_ref()
                            .map(|pulse| pulse.amplitude)
                            .unwrap_or(0.0),
                        pulse_speed: rule
                            .pulse
                            .as_ref()
                            .map(|pulse| pulse.speed.max(0.01))
                            .unwrap_or(1.0),
                        phase,
                        max_distance_squared: max_distance * max_distance,
                    },
                    NotShadowCaster,
                    NotShadowReceiver,
                    PbrBundle {
                        mesh: quad_mesh.clone(),
                        material: material_handle,
                        transform: Transform::from_scale(Vec3::splat(rule.size.max(1.0))),
                        visibility: Visibility::Hidden,
                        ..default()
                    },
                ));
                spawned_sprites += 1;
            }
        }

        commands.entity(entity).insert(MapVfxObjectApplied);
    }

    if applied_overrides > 0 || spawned_sprites > 0 {
        info!(
            "Map VFX '{}' applied to scene objects: overrides={}, sprites={}",
            world.world_name, applied_overrides, spawned_sprites
        );
    }
}

/// Keep map sprite effects camera-facing and distance-culled.
pub fn update_map_vfx_billboard_sprites(
    time: Res<Time>,
    camera_query: Query<&GlobalTransform, With<Camera3d>>,
    anchors: Query<&GlobalTransform, With<SceneObject>>,
    mut sprites: Query<(&MapVfxBillboardSprite, &mut Transform, &mut Visibility)>,
) {
    let Ok(camera_transform) = camera_query.get_single() else {
        return;
    };
    let (_, camera_rotation, camera_position) = camera_transform.to_scale_rotation_translation();
    let elapsed = time.elapsed_seconds();

    for (sprite, mut transform, mut visibility) in &mut sprites {
        let Ok(anchor_transform) = anchors.get(sprite.anchor) else {
            *visibility = Visibility::Hidden;
            continue;
        };

        let anchor_position = anchor_transform.translation() + Vec3::Y * sprite.vertical_offset;
        let distance_squared = anchor_position.distance_squared(camera_position);
        if distance_squared > sprite.max_distance_squared {
            *visibility = Visibility::Hidden;
            continue;
        }

        let pulse = 1.0
            + sprite.pulse_amplitude.max(0.0) * (elapsed * sprite.pulse_speed + sprite.phase).sin();
        let scale = (sprite.base_size * pulse.max(0.1)).max(1.0);
        let spin = elapsed * sprite.spin_speed + sprite.phase;

        transform.translation = anchor_position;
        transform.rotation = camera_rotation * Quat::from_rotation_z(spin);
        transform.scale = Vec3::new(scale, scale, 1.0);
        *visibility = Visibility::Inherited;
    }
}

fn consume_rule_budget(
    spawn_stride: u32,
    max_instances: Option<u32>,
    state: &mut RuleBudgetState,
) -> bool {
    state.seen = state.seen.saturating_add(1);
    let stride = spawn_stride.max(1);
    if (state.seen - 1) % stride != 0 {
        return false;
    }
    if let Some(max_instances) = max_instances {
        if state.applied >= max_instances {
            return false;
        }
    }
    state.applied = state.applied.saturating_add(1);
    true
}

fn sanitize_color(raw: [f32; 4]) -> [f32; 4] {
    [
        raw[0].clamp(0.0, 1.0),
        raw[1].clamp(0.0, 1.0),
        raw[2].clamp(0.0, 1.0),
        raw[3].clamp(0.0, 1.0),
    ]
}

fn quantize_color(color: [f32; 4]) -> [u8; 4] {
    [
        (color[0] * 255.0).round() as u8,
        (color[1] * 255.0).round() as u8,
        (color[2] * 255.0).round() as u8,
        (color[3] * 255.0).round() as u8,
    ]
}

fn phase_from_entity(entity: Entity, salt: u32) -> f32 {
    let seed = entity
        .index()
        .wrapping_mul(1103515245)
        .wrapping_add(12345 ^ salt);
    (seed % 10_000) as f32 * (TAU / 10_000.0)
}

fn normalize_existing_asset_path(raw_path: &str) -> Option<String> {
    const CLIENT_ASSETS_ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../assets");

    let normalized = raw_path
        .trim()
        .replace('\\', "/")
        .trim_start_matches('/')
        .to_string();
    if normalized.is_empty() {
        return None;
    }
    let full = Path::new(CLIENT_ASSETS_ROOT).join(&normalized);
    if full.is_file() {
        return Some(normalized);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::de::DeserializeOwned;
    use std::fs;

    const CLIENT_ASSETS_ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../assets");

    fn load_json_asset<T: DeserializeOwned>(relative_path: &str) -> T {
        let full_path = Path::new(CLIENT_ASSETS_ROOT).join(relative_path);
        let bytes = fs::read(&full_path)
            .unwrap_or_else(|err| panic!("failed to read asset {}: {err}", full_path.display()));
        serde_json::from_slice(&bytes)
            .unwrap_or_else(|err| panic!("failed to parse JSON {}: {err}", full_path.display()))
    }

    #[test]
    fn world4_vfx_profile_has_expected_rules() {
        let profile: MapVfxProfile = load_json_asset("data/world4/map_vfx.json");
        assert!(
            profile
                .object_overrides
                .iter()
                .any(|rule| rule.object_type == 19
                    && rule.particle_emitter.as_deref() == Some("losttower_fire")),
            "world4 map_vfx must configure object type 19 fire emitter"
        );
        assert!(
            profile
                .object_sprites
                .iter()
                .any(|rule| rule.object_type == 20 && rule.blend_mode == MapVfxBlendMode::Additive),
            "world4 map_vfx must configure additive sprite for object type 20"
        );
    }

    #[test]
    fn rule_budget_respects_stride_and_cap() {
        let mut state = RuleBudgetState::default();
        let mut applied = 0u32;
        for _ in 0..10 {
            if consume_rule_budget(3, Some(2), &mut state) {
                applied += 1;
            }
        }
        assert_eq!(applied, 2);
    }
}
