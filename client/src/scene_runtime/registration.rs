use crate::scene_runtime::components::{
    SceneObjectAnimationInitialized, SceneObjectAnimationSource,
};
use crate::scene_runtime::pipeline::SceneRenderPipeline;
use crate::scene_runtime::systems::{
    DebugFrameLimiter, DebugFreeCameraController, DebugOverlayState, DebugSceneStats,
    DebugShadowQuality, DynamicLightBudget, GrassMaterial, SceneObjectDistanceCullingConfig,
    animate_world_56_dark_lord, animate_world_56_flying_monsters,
    animate_world_56_sky_vortex_objects, animate_world_56_skybox, apply_debug_overlay_visibility,
    apply_grass_distance_culling, apply_grass_visibility_from_settings,
    apply_legacy_gltf_material_overrides, apply_map_vfx_profile_to_scene_objects,
    apply_scene_object_distance_culling, control_debug_free_camera, cycle_debug_frame_limit,
    cycle_debug_shadow_quality, draw_runtime_map_grid, ensure_particle_render_batches,
    ensure_scene_object_animation_players, fix_scene_object_materials, handle_window_occlusion,
    initialize_world_56_login_fx, load_scene_runtime_assets, reset_debug_free_camera,
    reset_debug_overlay_state, reset_debug_scene_stats, setup_camera_tour,
    spawn_boundary_walls_when_ready, spawn_debug_free_camera_hint, spawn_debug_scene_stats_hud,
    spawn_dynamic_lights, spawn_runtime_sun_light, spawn_scene_objects_when_ready,
    spawn_skybox_when_ready, spawn_terrain_grass_when_ready, spawn_terrain_when_ready,
    spawn_world_56_meteors, toggle_debug_free_camera, toggle_debug_overlay_shortcut,
    toggle_offscreen_scene_animations, update_boids, update_camera_tour,
    update_debug_free_camera_hint, update_debug_scene_stats, update_dynamic_lights,
    update_map_vfx_billboard_sprites, update_particle_emitters, update_particle_render_batches,
    update_world_56_meteors,
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

pub fn register_scene_runtime(app: &mut App) {
    app.add_plugins(MaterialPlugin::<GrassMaterial>::default())
        .add_systems(Startup, configure_runtime_gizmos)
        .init_resource::<DebugOverlayState>()
        .init_resource::<DebugSceneStats>()
        .init_resource::<DynamicLightBudget>()
        .init_resource::<SceneObjectDistanceCullingConfig>()
        .configure_sets(
            Update,
            (
                SceneRenderPipeline::Load,
                SceneRenderPipeline::Spawn,
                SceneRenderPipeline::Simulate,
                SceneRenderPipeline::Lighting,
                SceneRenderPipeline::Camera,
            )
                .chain(),
        )
        .add_systems(
            Update,
            load_scene_runtime_assets
                .in_set(SceneRenderPipeline::Load)
                .run_if(runtime_state_is_active),
        )
        .add_systems(
            Update,
            (
                spawn_terrain_when_ready,
                spawn_terrain_grass_when_ready,
                spawn_scene_objects_when_ready,
                spawn_skybox_when_ready,
                spawn_runtime_sun_light,
                spawn_boundary_walls_when_ready,
                setup_camera_tour,
                initialize_world_56_login_fx,
            )
                .in_set(SceneRenderPipeline::Spawn)
                .run_if(runtime_state_is_active),
        )
        .add_systems(
            Update,
            (
                apply_legacy_gltf_material_overrides,
                fix_scene_object_materials,
                ensure_scene_object_animation_players.run_if(
                    |q: Query<
                        (),
                        (
                            With<SceneObjectAnimationSource>,
                            Without<SceneObjectAnimationInitialized>,
                        ),
                    >| { !q.is_empty() },
                ),
                update_boids,
                apply_map_vfx_profile_to_scene_objects,
                animate_world_56_skybox,
                animate_world_56_sky_vortex_objects,
                animate_world_56_flying_monsters,
                spawn_world_56_meteors,
                update_world_56_meteors,
                animate_world_56_dark_lord,
            )
                .in_set(SceneRenderPipeline::Simulate)
                .run_if(runtime_state_is_active),
        )
        .add_systems(
            Update,
            (
                update_particle_emitters,
                ensure_particle_render_batches,
                update_particle_render_batches,
                update_map_vfx_billboard_sprites,
            )
                .chain()
                .in_set(SceneRenderPipeline::Simulate)
                .run_if(runtime_state_is_active),
        )
        .add_systems(
            Update,
            (spawn_dynamic_lights, update_dynamic_lights)
                .in_set(SceneRenderPipeline::Lighting)
                .run_if(runtime_state_is_active),
        )
        .add_systems(
            OnEnter(crate::AppState::Gameplay),
            (
                reset_debug_overlay_state,
                reset_debug_scene_stats,
                spawn_debug_scene_stats_hud,
            ),
        )
        .add_systems(
            Update,
            (
                update_camera_tour,
                apply_scene_object_distance_culling,
                apply_grass_distance_culling,
                apply_grass_visibility_from_settings,
                toggle_offscreen_scene_animations,
                draw_runtime_map_grid,
            )
                .chain()
                .in_set(SceneRenderPipeline::Camera)
                .run_if(runtime_state_is_active),
        )
        .add_systems(
            Update,
            (
                toggle_debug_overlay_shortcut,
                apply_debug_overlay_visibility,
            )
                .in_set(SceneRenderPipeline::Camera)
                .run_if(runtime_state_is_active),
        )
        .add_systems(
            Update,
            update_debug_scene_stats
                .in_set(SceneRenderPipeline::Camera)
                .run_if(runtime_state_is_active)
                .run_if(|s: Res<DebugOverlayState>| s.visible),
        );

    app.add_systems(Update, handle_window_occlusion);

    if cfg!(debug_assertions) {
        app.init_resource::<DebugFreeCameraController>()
            .init_resource::<DebugFrameLimiter>()
            .init_resource::<DebugShadowQuality>()
            .add_systems(
                OnEnter(crate::AppState::Gameplay),
                (reset_debug_free_camera, spawn_debug_free_camera_hint),
            )
            .add_systems(
                Update,
                (
                    toggle_debug_free_camera,
                    control_debug_free_camera,
                    update_debug_free_camera_hint,
                    cycle_debug_frame_limit,
                    cycle_debug_shadow_quality,
                )
                    .in_set(SceneRenderPipeline::Camera)
                    .run_if(runtime_state_is_active),
            );
    }
}
