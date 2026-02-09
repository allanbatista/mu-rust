use crate::scene_runtime::pipeline::SceneRenderPipeline;
use crate::scene_runtime::systems::{
    DebugFreeCameraController, DebugSceneStats, control_debug_free_camera,
    ensure_scene_object_animation_players, load_scene_runtime_assets, reset_debug_free_camera,
    reset_debug_scene_stats, setup_camera_tour, spawn_debug_free_camera_hint,
    spawn_debug_scene_stats_hud, spawn_dynamic_lights, spawn_runtime_sun_light,
    spawn_scene_objects_when_ready, spawn_terrain_when_ready, toggle_debug_free_camera,
    update_boids, update_camera_tour, update_debug_free_camera_hint, update_debug_scene_stats,
    update_dynamic_lights, update_particle_emitters,
};
use bevy::prelude::*;
use bevy::state::prelude::{OnEnter, States, in_state};

pub fn register_scene_runtime<S: States + Copy>(app: &mut App, active_state: S) {
    app.configure_sets(
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
            spawn_scene_objects_when_ready,
            spawn_runtime_sun_light,
            setup_camera_tour,
        )
            .in_set(SceneRenderPipeline::Spawn)
            .run_if(in_state(active_state)),
    )
    .add_systems(
        Update,
        (
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
        Update,
        update_camera_tour
            .in_set(SceneRenderPipeline::Camera)
            .run_if(in_state(active_state)),
    );

    if cfg!(debug_assertions) {
        app.init_resource::<DebugFreeCameraController>()
            .init_resource::<DebugSceneStats>()
            .add_systems(
                OnEnter(active_state),
                (
                    reset_debug_free_camera,
                    spawn_debug_free_camera_hint,
                    reset_debug_scene_stats,
                    spawn_debug_scene_stats_hud,
                ),
            )
            .add_systems(
                Update,
                (
                    toggle_debug_free_camera,
                    control_debug_free_camera,
                    update_debug_free_camera_hint,
                    update_debug_scene_stats,
                )
                    .in_set(SceneRenderPipeline::Camera)
                    .run_if(in_state(active_state)),
            );
    }
}
