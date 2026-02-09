use crate::scene_runtime::systems::RuntimeSunLight;
use bevy::pbr::{CascadeShadowConfig, CascadeShadowConfigBuilder, DirectionalLightShadowMap, ShadowFilteringMethod};
use bevy::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShadowQualityMode {
    Low,
    Medium,
    High,
    Off,
}

impl std::fmt::Display for ShadowQualityMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShadowQualityMode::Low => write!(f, "Baixa (Hardware2x2, 1 cascade, 1024)"),
            ShadowQualityMode::Medium => write!(f, "Média (Gaussian, 2 cascades, 2048)"),
            ShadowQualityMode::High => write!(f, "Alta (Gaussian, 3 cascades, 4096)"),
            ShadowQualityMode::Off => write!(f, "Desativada"),
        }
    }
}

#[derive(Resource)]
pub struct DebugShadowQuality {
    pub mode: ShadowQualityMode,
}

impl Default for DebugShadowQuality {
    fn default() -> Self {
        Self {
            mode: ShadowQualityMode::Low,
        }
    }
}

pub fn cycle_debug_shadow_quality(
    keys: Res<ButtonInput<KeyCode>>,
    mut shadow_quality: ResMut<DebugShadowQuality>,
    mut sun_query: Query<(&mut DirectionalLight, &mut CascadeShadowConfig), With<RuntimeSunLight>>,
    mut shadow_map: ResMut<DirectionalLightShadowMap>,
    camera_query: Query<Entity, With<Camera3d>>,
    mut commands: Commands,
) {
    if !keys.just_pressed(KeyCode::F5) {
        return;
    }

    shadow_quality.mode = match shadow_quality.mode {
        ShadowQualityMode::Low => ShadowQualityMode::Medium,
        ShadowQualityMode::Medium => ShadowQualityMode::High,
        ShadowQualityMode::High => ShadowQualityMode::Off,
        ShadowQualityMode::Off => ShadowQualityMode::Low,
    };

    let mode = shadow_quality.mode;

    for (mut light, mut cascade_config) in sun_query.iter_mut() {
        match mode {
            ShadowQualityMode::Low => {
                light.shadows_enabled = true;
                shadow_map.size = 1024;
                *cascade_config = CascadeShadowConfigBuilder {
                    num_cascades: 1,
                    minimum_distance: 10.0,
                    maximum_distance: 8_000.0,
                    first_cascade_far_bound: 8_000.0,
                    overlap_proportion: 0.15,
                }
                .into();
            }
            ShadowQualityMode::Medium => {
                light.shadows_enabled = true;
                shadow_map.size = 2048;
                *cascade_config = CascadeShadowConfigBuilder {
                    num_cascades: 2,
                    minimum_distance: 10.0,
                    maximum_distance: 8_000.0,
                    first_cascade_far_bound: 1_200.0,
                    overlap_proportion: 0.15,
                }
                .into();
            }
            ShadowQualityMode::High => {
                light.shadows_enabled = true;
                shadow_map.size = 4096;
                *cascade_config = CascadeShadowConfigBuilder {
                    num_cascades: 3,
                    minimum_distance: 10.0,
                    maximum_distance: 8_000.0,
                    first_cascade_far_bound: 800.0,
                    overlap_proportion: 0.15,
                }
                .into();
            }
            ShadowQualityMode::Off => {
                light.shadows_enabled = false;
            }
        }
    }

    // Update ShadowFilteringMethod on camera
    if mode != ShadowQualityMode::Off {
        let filtering = match mode {
            ShadowQualityMode::Low => ShadowFilteringMethod::Hardware2x2,
            _ => ShadowFilteringMethod::Gaussian,
        };
        for entity in camera_query.iter() {
            commands.entity(entity).insert(filtering);
        }
    }

    info!("Shadow quality: {}", mode);
}
