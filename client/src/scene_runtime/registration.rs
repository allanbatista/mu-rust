use crate::scene_runtime::pipeline::SceneRenderPipeline;
use crate::scene_runtime::systems::{
    DebugFreeCameraController, DebugOverlayState, DebugSceneStats, GrassMaterial,
    SceneObjectDistanceCullingConfig, apply_debug_overlay_visibility,
    apply_legacy_gltf_material_overrides, apply_scene_object_distance_culling,
    control_debug_free_camera, ensure_scene_object_animation_players, load_scene_runtime_assets,
    reset_debug_free_camera, reset_debug_overlay_state, reset_debug_scene_stats, setup_camera_tour,
    spawn_debug_free_camera_hint, spawn_debug_scene_stats_hud, spawn_dynamic_lights,
    spawn_runtime_sun_light, spawn_scene_objects_when_ready, spawn_skybox_when_ready,
    spawn_terrain_grass_when_ready, spawn_terrain_when_ready, toggle_debug_free_camera,
    toggle_debug_overlay_shortcut, update_boids, update_camera_tour, update_debug_free_camera_hint,
    update_debug_scene_stats, update_dynamic_lights, update_particle_emitters,
};
use bevy::pbr::MaterialPlugin;
use bevy::prelude::*;
use bevy::state::prelude::{OnEnter, States, in_state};

pub fn register_scene_runtime<S: States + Copy>(app: &mut App, active_state: S) {
    app.add_plugins(MaterialPlugin::<GrassMaterial>::default())
        .init_resource::<DebugOverlayState>()
        .init_resource::<DebugSceneStats>()
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
                .run_if(in_state(active_state)),
        )
        .add_systems(
            Update,
            (
                spawn_terrain_when_ready,
                spawn_terrain_grass_when_ready,
                spawn_scene_objects_when_ready,
                spawn_skybox_when_ready,
                spawn_runtime_sun_light,
                setup_camera_tour,
            )
                .in_set(SceneRenderPipeline::Spawn)
                .run_if(in_state(active_state)),
        )
        .add_systems(
            Update,
            (
                apply_legacy_gltf_material_overrides,
                ensure_scene_object_animation_players,
                update_boids,
                update_particle_emitters,
            )
                .in_set(SceneRenderPipeline::Simulate)
                .run_if(in_state(active_state)),
        )
        .add_systems(
            Update,
            (spawn_dynamic_lights, update_dynamic_lights)
                .in_set(SceneRenderPipeline::Lighting)
                .run_if(in_state(active_state)),
        )
        .add_systems(
            OnEnter(active_state),
            (
                reset_debug_overlay_state,
                reset_debug_scene_stats,
                spawn_debug_scene_stats_hud,
            ),
        )
        .add_systems(
            Update,
            (update_camera_tour, apply_scene_object_distance_culling)
                .chain()
                .in_set(SceneRenderPipeline::Camera)
                .run_if(in_state(active_state)),
        )
        .add_systems(
            Update,
            (
                toggle_debug_overlay_shortcut,
                apply_debug_overlay_visibility,
                update_debug_scene_stats,
            )
                .in_set(SceneRenderPipeline::Camera)
                .run_if(in_state(active_state)),
        );

    if cfg!(debug_assertions) {
        app.init_resource::<DebugFreeCameraController>()
            .add_systems(
                OnEnter(active_state),
                (reset_debug_free_camera, spawn_debug_free_camera_hint),
            )
            .add_systems(
                Update,
                (
                    toggle_debug_free_camera,
                    control_debug_free_camera,
                    update_debug_free_camera_hint,
                )
                    .in_set(SceneRenderPipeline::Camera)
                    .run_if(in_state(active_state)),
            );
    }
}
