use crate::scenes::login::components::*;
use bevy::prelude::*;

/// Marker for spawned point lights
#[derive(Component)]
pub struct DynamicPointLight;

/// System to spawn point lights for dynamic light components
pub fn spawn_dynamic_lights(
    mut commands: Commands,
    objects: Query<(Entity, &DynamicLight, &Transform), Added<DynamicLight>>,
) {
    for (_entity, dynamic_light, transform) in objects.iter() {
        // Spawn a point light as a child of the object
        commands.spawn((
            LoginSceneEntity,
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
