use crate::bevy_compat::*;
use crate::scene_runtime::components::*;
use crate::scene_runtime::state::RuntimeSceneAssets;
use bevy::light::{
    CascadeShadowConfigBuilder, DirectionalLightShadowMap, GlobalAmbientLight,
    ShadowFilteringMethod,
};
use bevy::prelude::*;
use std::cmp::Ordering;

const DEFAULT_MAX_DYNAMIC_POINT_LIGHTS: usize = 12;
const DEFAULT_DYNAMIC_LIGHT_MAX_DISTANCE: f32 = 2400.0;
const DYNAMIC_LIGHT_POOL_ENV: &str = "MU_DYNAMIC_LIGHT_POOL";
const DYNAMIC_LIGHT_DISTANCE_ENV: &str = "MU_DYNAMIC_LIGHT_MAX_DISTANCE";

/// Marker for spawned point lights
#[derive(Component)]
pub struct DynamicPointLight;

#[derive(Component, Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct DynamicPointLightSlot(pub usize);

/// Marker for the runtime directional "sun" light.
#[derive(Component)]
pub struct RuntimeSunLight;

/// Marker for secondary "fill" light used to soften harsh shadows.
#[derive(Component)]
pub struct RuntimeFillLight;

#[derive(Resource, Clone, Debug)]
pub struct DynamicLightBudget {
    pub max_active_lights: usize,
    pub max_distance: f32,
    max_distance_squared: f32,
}

impl Default for DynamicLightBudget {
    fn default() -> Self {
        let max_active_lights = std::env::var(DYNAMIC_LIGHT_POOL_ENV)
            .ok()
            .and_then(|raw| raw.trim().parse::<usize>().ok())
            .map(|value| value.clamp(0, 64))
            .unwrap_or(DEFAULT_MAX_DYNAMIC_POINT_LIGHTS);

        let max_distance = std::env::var(DYNAMIC_LIGHT_DISTANCE_ENV)
            .ok()
            .and_then(|raw| raw.trim().parse::<f32>().ok())
            .filter(|value| value.is_finite() && *value > 0.0)
            .unwrap_or(DEFAULT_DYNAMIC_LIGHT_MAX_DISTANCE);

        Self {
            max_active_lights,
            max_distance,
            max_distance_squared: max_distance * max_distance,
        }
    }
}

/// Spawn one central sun light for the active runtime world.
pub fn spawn_runtime_sun_light(
    mut commands: Commands,
    assets: Res<RuntimeSceneAssets>,
    terrain_configs: Res<Assets<TerrainConfig>>,
    mut ambient_light: ResMut<GlobalAmbientLight>,
    query: Query<Entity, With<RuntimeSunLight>>,
    camera_query: Query<Entity, With<Camera3d>>,
) {
    if !assets.loaded || !query.is_empty() {
        return;
    }

    let Some(world) = assets.world.as_ref() else {
        return;
    };

    let Some(config) = terrain_configs.get(&world.terrain_config) else {
        return;
    };

    let center_x = config.size.width as f32 * config.size.scale * 0.5;
    let center_z = config.size.depth as f32 * config.size.scale * 0.5;
    let target = Vec3::new(center_x, 0.0, center_z);
    let origin = target + Vec3::new(0.0, 14_000.0, 9_000.0);
    let fill_origin = target + Vec3::new(0.0, 8_000.0, -9_000.0);

    commands.spawn((
        RuntimeSceneEntity,
        RuntimeSunLight,
        DirectionalLightBundle {
            directional_light: DirectionalLight {
                color: Color::srgb(1.0, 0.98, 0.94),
                illuminance: 5000.0,
                shadows_enabled: true,
                ..default()
            },
            cascade_shadow_config: CascadeShadowConfigBuilder {
                num_cascades: 1,
                minimum_distance: 10.0,
                maximum_distance: 8_000.0,
                first_cascade_far_bound: 8_000.0,
                overlap_proportion: 0.15,
            }
            .into(),
            transform: Transform::from_translation(origin).looking_at(target, Vec3::Y),
            ..default()
        },
    ));

    // A weaker opposite-side fill light to brighten dark faces without extra shadow cost.
    commands.spawn((
        RuntimeSceneEntity,
        RuntimeFillLight,
        DirectionalLightBundle {
            directional_light: DirectionalLight {
                color: Color::srgb(0.9, 0.94, 1.0),
                illuminance: 2500.0,
                shadows_enabled: false,
                ..default()
            },
            transform: Transform::from_translation(fill_origin).looking_at(target, Vec3::Y),
            ..default()
        },
    ));

    // Lift shadowed areas so terrain/object shadows remain visible but not crushed to black.
    ambient_light.color = Color::srgb(0.96, 0.97, 1.0);
    ambient_light.brightness = 0.55;
    ambient_light.affects_lightmapped_meshes = true;

    // Set initial shadow quality to Low
    commands.insert_resource(DirectionalLightShadowMap { size: 1024 });
    for entity in camera_query.iter() {
        commands
            .entity(entity)
            .insert(ShadowFilteringMethod::Hardware2x2);
    }

    info!(
        "Runtime sun spawned for '{}' (center: {:.1}, {:.1})",
        world.world_name, center_x, center_z
    );
}

/// System to spawn point lights for dynamic light components
pub fn spawn_dynamic_lights(
    mut commands: Commands,
    budget: Res<DynamicLightBudget>,
    lights: Query<(Entity, &DynamicPointLightSlot), With<DynamicPointLight>>,
) {
    if budget.is_changed() {
        info!(
            "Dynamic light pool budget: pool={}, max_distance={:.0}",
            budget.max_active_lights, budget.max_distance
        );
    }

    let mut existing: Vec<(Entity, usize)> = lights
        .iter()
        .map(|(entity, slot)| (entity, slot.0))
        .collect();
    existing.sort_by_key(|(_, slot)| *slot);

    if existing.len() > budget.max_active_lights {
        for (entity, _) in existing.iter().skip(budget.max_active_lights) {
            commands.entity(*entity).despawn();
        }
        existing.truncate(budget.max_active_lights);
    }

    if existing.len() >= budget.max_active_lights {
        return;
    }

    for slot in existing.len()..budget.max_active_lights {
        commands.spawn((
            RuntimeSceneEntity,
            DynamicPointLight,
            DynamicPointLightSlot(slot),
            PointLightBundle {
                point_light: PointLight {
                    intensity: 0.0,
                    range: 0.0,
                    shadows_enabled: false,
                    ..default()
                },
                transform: Transform::IDENTITY,
                ..default()
            },
        ));
    }
}

/// Select the closest and most relevant dynamic lights and project them to a small light pool.
pub fn update_dynamic_lights(
    budget: Res<DynamicLightBudget>,
    camera_query: Query<&GlobalTransform, With<Camera3d>>,
    dynamic_lights: Query<
        (&DynamicLight, &GlobalTransform, Option<&Visibility>),
        Without<DynamicPointLight>,
    >,
    mut light_pool: Query<
        (
            &DynamicPointLightSlot,
            &mut PointLight,
            &mut Transform,
            &mut Visibility,
        ),
        With<DynamicPointLight>,
    >,
    time: Res<Time>,
) {
    let Ok(camera_transform) = camera_query.single() else {
        return;
    };
    let (_, camera_rotation, camera_position) = camera_transform.to_scale_rotation_translation();
    let camera_forward = camera_rotation * -Vec3::Z;

    let mut candidates: Vec<(f32, Vec3, DynamicLight)> = Vec::new();
    candidates.reserve(budget.max_active_lights.saturating_mul(8));

    for (dynamic_light, transform, visibility) in &dynamic_lights {
        if matches!(visibility, Some(Visibility::Hidden)) {
            continue;
        }

        let position = transform.translation();
        let distance_squared = position.distance_squared(camera_position);
        if distance_squared > budget.max_distance_squared {
            continue;
        }

        // Penalize lights behind the camera; still allow some to avoid abrupt popping.
        let direction = (position - camera_position).normalize_or_zero();
        let facing = camera_forward.dot(direction);
        let back_penalty = if facing < -0.2 {
            budget.max_distance_squared
        } else if facing < 0.0 {
            budget.max_distance_squared * 0.25
        } else {
            0.0
        };

        candidates.push((
            distance_squared + back_penalty,
            position,
            dynamic_light.clone(),
        ));
    }

    candidates.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(Ordering::Equal));

    let mut selected = candidates
        .into_iter()
        .take(budget.max_active_lights)
        .map(|(_, position, light)| (position, light));

    let mut pool: Vec<(usize, Mut<PointLight>, Mut<Transform>, Mut<Visibility>)> = light_pool
        .iter_mut()
        .map(|(slot, point_light, transform, visibility)| {
            (slot.0, point_light, transform, visibility)
        })
        .collect();
    pool.sort_by_key(|(slot, _, _, _)| *slot);

    for (_, mut point_light, mut transform, mut visibility) in pool {
        if let Some((position, dynamic_light)) = selected.next() {
            point_light.color = dynamic_light.color;
            point_light.range = dynamic_light.range;
            point_light.intensity = if let Some(flicker) = &dynamic_light.flicker {
                let flicker_value = ((time.elapsed_secs() * flicker.speed).sin() + 1.0) / 2.0;
                let intensity = flicker.min_intensity
                    + (flicker.max_intensity - flicker.min_intensity) * flicker_value;
                dynamic_light.intensity * intensity
            } else {
                dynamic_light.intensity
            };

            transform.translation = position;
            transform.rotation = Quat::IDENTITY;
            *visibility = Visibility::Inherited;
        } else {
            point_light.intensity = 0.0;
            point_light.range = 0.0;
            *visibility = Visibility::Hidden;
        }
    }
}
