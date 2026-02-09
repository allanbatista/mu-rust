use crate::scene_runtime::components::*;
use crate::scene_runtime::state::RuntimeSceneAssets;
use bevy::pbr::{CascadeShadowConfigBuilder, DirectionalLightShadowMap, ShadowFilteringMethod};
use bevy::prelude::*;

/// Marker for spawned point lights
#[derive(Component)]
pub struct DynamicPointLight;

/// Marker for the runtime directional "sun" light.
#[derive(Component)]
pub struct RuntimeSunLight;

/// Marker for secondary "fill" light used to soften harsh shadows.
#[derive(Component)]
pub struct RuntimeFillLight;

/// Spawn one central sun light for the active runtime world.
pub fn spawn_runtime_sun_light(
    mut commands: Commands,
    assets: Res<RuntimeSceneAssets>,
    terrain_configs: Res<Assets<TerrainConfig>>,
    mut ambient_light: ResMut<AmbientLight>,
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
                illuminance: 9000.0,
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
    objects: Query<(Entity, &DynamicLight, &Transform), Added<DynamicLight>>,
) {
    for (_entity, dynamic_light, transform) in objects.iter() {
        // Spawn a point light as a child of the object
        commands.spawn((
            RuntimeSceneEntity,
            DynamicPointLight,
            dynamic_light.clone(),
            PointLightBundle {
                point_light: PointLight {
                    color: dynamic_light.color,
                    intensity: dynamic_light.intensity,
                    range: dynamic_light.range,
                    shadows_enabled: false, // Performance optimization
                    ..default()
                },
                transform: *transform,
                ..default()
            },
        ));

        // Note: In a more complex system, you might want to parent the light to the object
        // For now, we'll just update positions in the update system
    }
}

/// System to update dynamic lights (flicker effect)
pub fn update_dynamic_lights(
    mut lights: Query<(&DynamicLight, &mut PointLight), With<DynamicPointLight>>,
    time: Res<Time>,
) {
    for (dynamic_light, mut point_light) in lights.iter_mut() {
        // Apply base color and range
        point_light.color = dynamic_light.color;
        point_light.range = dynamic_light.range;

        // Apply flicker if configured
        if let Some(flicker) = &dynamic_light.flicker {
            let flicker_value = ((time.elapsed_seconds() * flicker.speed).sin() + 1.0) / 2.0;
            let intensity = flicker.min_intensity
                + (flicker.max_intensity - flicker.min_intensity) * flicker_value;
            point_light.intensity = dynamic_light.intensity * intensity;
        } else {
            point_light.intensity = dynamic_light.intensity;
        }
    }
}
