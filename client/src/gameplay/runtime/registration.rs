use crate::gameplay::runtime::pipeline::GameplayPipelineSet;
use crate::gameplay::systems::{camera, lighting, objects, particles, skills, terrain};
use crate::legacy_additive::LegacyAdditiveMaterial;
use crate::scene_runtime::components::{
    SceneObjectAnimationInitialized, SceneObjectAnimationSource,
};
use crate::scene_runtime::systems::{
    DynamicLightBudget, GrassMaterial, SceneObjectDistanceCullingConfig,
    animate_world_56_dark_lord, animate_world_56_flying_monsters,
    animate_world_56_sky_vortex_objects, animate_world_56_skybox, handle_window_occlusion,
    initialize_world_56_login_fx, load_scene_runtime_assets, spawn_skybox_when_ready,
    spawn_world_56_meteors, update_boids, update_world_56_meteors,
};
use bevy::gizmos::config::{DefaultGizmoConfigGroup, GizmoConfigStore};
use bevy::pbr::MaterialPlugin;
use bevy::prelude::*;
use bevy::state::prelude::OnEnter;

fn runtime_state_is_active(state: Res<State<crate::AppState>>) -> bool {
    matches!(state.get(), crate::AppState::Gameplay)
}

fn configure_runtime_gizmos(mut config_store: ResMut<GizmoConfigStore>) {
    let (config, _) = config_store.config_mut::<DefaultGizmoConfigGroup>();
    config.enabled = true;
    config.depth_bias = -1.0;
    config.line.width = 3.0;
}

pub fn register_gameplay_runtime(app: &mut App) {
    app.add_plugins(MaterialPlugin::<GrassMaterial>::default())
        .add_plugins(MaterialPlugin::<LegacyAdditiveMaterial>::default())
        .add_systems(Startup, configure_runtime_gizmos)
        .init_resource::<camera::DebugOverlayState>()
        .init_resource::<camera::DebugSceneStats>()
        .init_resource::<DynamicLightBudget>()
        .init_resource::<SceneObjectDistanceCullingConfig>()
        .configure_sets(
            Update,
            (
                GameplayPipelineSet::AssetLoad,
                GameplayPipelineSet::WorldSpawn,
                GameplayPipelineSet::WorldSimulate,
                GameplayPipelineSet::EffectsSimulate,
                GameplayPipelineSet::Lighting,
                GameplayPipelineSet::Camera,
                GameplayPipelineSet::UiSync,
            )
                .chain(),
        )
        .add_systems(
            Update,
            load_scene_runtime_assets
                .in_set(GameplayPipelineSet::AssetLoad)
                .run_if(runtime_state_is_active),
        )
        .add_systems(
            Update,
            (
                terrain::spawn_terrain_when_ready,
                terrain::spawn_terrain_grass_when_ready,
                objects::spawn_scene_objects_when_ready,
                spawn_skybox_when_ready,
                lighting::spawn_runtime_sun_light,
                terrain::spawn_boundary_walls_when_ready,
                camera::setup_camera_tour,
                initialize_world_56_login_fx,
            )
                .in_set(GameplayPipelineSet::WorldSpawn)
                .run_if(runtime_state_is_active),
        )
        .add_systems(
            Update,
            (
                objects::apply_legacy_gltf_material_overrides,
                objects::fix_scene_object_materials,
                objects::ensure_scene_object_animation_players.run_if(
                    |q: Query<
                        (),
                        (
                            With<SceneObjectAnimationSource>,
                            Without<SceneObjectAnimationInitialized>,
                        ),
                    >| { !q.is_empty() },
                ),
                update_boids,
                objects::apply_map_vfx_profile_to_scene_objects,
                animate_world_56_skybox,
                animate_world_56_sky_vortex_objects,
                animate_world_56_flying_monsters,
                spawn_world_56_meteors,
                update_world_56_meteors,
                animate_world_56_dark_lord,
            )
                .in_set(GameplayPipelineSet::WorldSimulate)
                .run_if(runtime_state_is_active),
        )
        .add_systems(
            Update,
            (
                skills::update_weapon_trails,
                skills::update_skill_impact_bursts,
                skills::update_skill_timed_lights,
                skills::update_death_stab_timeline,
                skills::apply_death_stab_vfx_materials,
                skills::ensure_death_stab_animation_players,
                skills::update_death_stab_energy_particles,
                skills::update_death_stab_spike_particles,
                skills::update_lightning_hurt_effects,
                skills::update_skill_vfx_auto_lifetimes,
                particles::update_particle_emitters,
                particles::ensure_particle_render_batches,
                particles::update_particle_render_batches,
                particles::update_map_vfx_billboard_sprites,
            )
                .chain()
                .in_set(GameplayPipelineSet::EffectsSimulate)
                .run_if(runtime_state_is_active),
        )
        .add_systems(
            Update,
            (
                lighting::spawn_dynamic_lights,
                lighting::update_dynamic_lights,
            )
                .in_set(GameplayPipelineSet::Lighting)
                .run_if(runtime_state_is_active),
        )
        .add_systems(
            OnEnter(crate::AppState::Gameplay),
            (
                camera::reset_debug_overlay_state,
                camera::reset_debug_scene_stats,
                camera::spawn_debug_scene_stats_hud,
            ),
        )
        .add_systems(
            Update,
            (
                camera::update_camera_tour,
                objects::apply_scene_object_distance_culling,
                terrain::apply_grass_distance_culling,
                terrain::apply_grass_visibility_from_settings,
                objects::toggle_offscreen_scene_animations,
                camera::draw_runtime_map_grid,
            )
                .chain()
                .in_set(GameplayPipelineSet::Camera)
                .run_if(runtime_state_is_active),
        )
        .add_systems(
            Update,
            (
                camera::toggle_debug_overlay_shortcut,
                camera::apply_debug_overlay_visibility,
            )
                .in_set(GameplayPipelineSet::UiSync)
                .run_if(runtime_state_is_active),
        )
        .add_systems(
            Update,
            camera::update_debug_scene_stats
                .in_set(GameplayPipelineSet::UiSync)
                .run_if(runtime_state_is_active)
                .run_if(|s: Res<camera::DebugOverlayState>| s.visible),
        );

    app.add_systems(Update, handle_window_occlusion);

    if cfg!(debug_assertions) {
        app.init_resource::<camera::DebugFreeCameraController>()
            .init_resource::<camera::DebugFrameLimiter>()
            .init_resource::<camera::DebugShadowQuality>()
            .add_systems(
                OnEnter(crate::AppState::Gameplay),
                (
                    camera::reset_debug_free_camera,
                    camera::spawn_debug_free_camera_hint,
                ),
            )
            .add_systems(
                Update,
                (
                    camera::toggle_debug_free_camera,
                    camera::control_debug_free_camera,
                    camera::update_debug_free_camera_hint,
                    camera::cycle_debug_frame_limit,
                    camera::cycle_debug_shadow_quality,
                )
                    .in_set(GameplayPipelineSet::Camera)
                    .run_if(runtime_state_is_active),
            );
    }
}
