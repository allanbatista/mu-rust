use super::frame_limiter::DebugFrameLimiter;
use super::objects::SceneObjectDistanceCullingConfig;
use super::shadow_quality::DebugShadowQuality;
use crate::bevy_compat::*;
use crate::scene_runtime::components::*;
use bevy::asset::AssetId;
use bevy::camera::visibility::VisibleEntities;
use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::ecs::system::SystemParam;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use bevy::render::renderer::RenderAdapterInfo;
use std::any::TypeId;
use std::collections::{HashMap, HashSet};

#[derive(SystemParam)]
pub struct DebugHudExtras<'w> {
    frame_limiter: Option<Res<'w, DebugFrameLimiter>>,
    shadow_quality: Option<Res<'w, DebugShadowQuality>>,
}

/// UI marker for performance stats text (fps/frame/object counters).
#[derive(Component)]
pub struct DebugSceneStatsPerformanceText;

/// UI marker for graphics adapter stats text.
#[derive(Component)]
pub struct DebugSceneStatsGpuText;

#[derive(Resource)]
pub struct DebugOverlayState {
    pub visible: bool,
}

impl Default for DebugOverlayState {
    fn default() -> Self {
        Self { visible: true }
    }
}

#[derive(Resource)]
pub struct DebugSceneStats {
    pub refresh_timer: Timer,
    pub object_count: usize,
    pub visible_object_count: usize,
    pub mesh_count: usize,
    pub visible_mesh_count: usize,
    pub polygon_count: u64,
    pub visible_polygon_count: u64,
    pub last_runtime_entity_count: usize,
    pub last_mesh_asset_count: usize,
}

impl Default for DebugSceneStats {
    fn default() -> Self {
        Self {
            refresh_timer: Timer::from_seconds(0.75, TimerMode::Repeating),
            object_count: 0,
            visible_object_count: 0,
            mesh_count: 0,
            visible_mesh_count: 0,
            polygon_count: 0,
            visible_polygon_count: 0,
            last_runtime_entity_count: 0,
            last_mesh_asset_count: 0,
        }
    }
}

pub fn reset_debug_scene_stats(mut debug_stats: ResMut<DebugSceneStats>) {
    *debug_stats = DebugSceneStats::default();
}

pub fn reset_debug_overlay_state(mut overlay_state: ResMut<DebugOverlayState>) {
    *overlay_state = DebugOverlayState::default();
}

pub fn toggle_debug_overlay_shortcut(
    keys: Res<ButtonInput<KeyCode>>,
    mut overlay_state: ResMut<DebugOverlayState>,
) {
    if keys.just_pressed(KeyCode::F3) {
        overlay_state.visible = !overlay_state.visible;
        info!(
            "Debug overlay {}",
            if overlay_state.visible {
                "enabled"
            } else {
                "disabled"
            }
        );
    }
}

pub fn apply_debug_overlay_visibility(
    overlay_state: Res<DebugOverlayState>,
    mut overlay_elements: Query<&mut Visibility, With<DebugOverlayElement>>,
) {
    if !overlay_state.is_changed() {
        return;
    }

    let visibility = if overlay_state.visible {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };
    for mut element_visibility in &mut overlay_elements {
        *element_visibility = visibility;
    }
}

pub fn spawn_debug_scene_stats_hud(mut commands: Commands) {
    let mut performance_text = TextBundle::from_section(
        "",
        TextStyle {
            font_size: 16.0,
            color: Color::WHITE,
            ..default()
        },
    )
    .with_text_justify(JustifyText::Right)
    .with_style(Style {
        position_type: PositionType::Absolute,
        top: Val::Px(14.0),
        right: Val::Px(14.0),
        ..default()
    });
    performance_text.background_color = BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.5));

    commands.spawn((
        RuntimeSceneEntity,
        DebugOverlayElement,
        DebugSceneStatsPerformanceText,
        performance_text,
    ));

    let mut gpu_text = TextBundle::from_section(
        "",
        TextStyle {
            font_size: 16.0,
            color: Color::WHITE,
            ..default()
        },
    )
    .with_style(Style {
        position_type: PositionType::Absolute,
        bottom: Val::Px(14.0),
        left: Val::Px(14.0),
        ..default()
    });
    gpu_text.background_color = BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.5));

    commands.spawn((
        RuntimeSceneEntity,
        DebugOverlayElement,
        DebugSceneStatsGpuText,
        gpu_text,
    ));
}

pub fn update_debug_scene_stats(
    time: Res<Time>,
    diagnostics: Res<DiagnosticsStore>,
    adapter_info: Option<Res<RenderAdapterInfo>>,
    meshes: Res<Assets<Mesh>>,
    login_entities: Query<Entity, With<RuntimeSceneEntity>>,
    scene_objects: Query<(), With<SceneObject>>,
    parent_query: Query<&ChildOf>,
    children_query: Query<&Children>,
    camera_visible_meshes: Query<&VisibleEntities, With<Camera3d>>,
    mesh_handles: Query<&Mesh3d>,
    distance_culling: Option<Res<SceneObjectDistanceCullingConfig>>,
    mut debug_stats: ResMut<DebugSceneStats>,
    overlay_state: Res<DebugOverlayState>,
    mut perf_text_query: Query<
        &mut Text,
        (
            With<DebugSceneStatsPerformanceText>,
            Without<DebugSceneStatsGpuText>,
        ),
    >,
    mut gpu_text_query: Query<
        &mut Text,
        (
            With<DebugSceneStatsGpuText>,
            Without<DebugSceneStatsPerformanceText>,
        ),
    >,
    debug_extras: DebugHudExtras,
) {
    if !overlay_state.visible {
        return;
    }

    debug_stats.refresh_timer.tick(time.delta());
    if cfg!(debug_assertions) && debug_stats.refresh_timer.just_finished() {
        let runtime_entity_count = login_entities.iter().count();
        let mesh_asset_count = meshes.len();

        let mut visited_entities = HashSet::<Entity>::new();
        let mut stack: Vec<Entity> = login_entities.iter().collect();
        let mut mesh_triangles_by_id = HashMap::<AssetId<Mesh>, u64>::new();
        let mut visible_objects = HashSet::<Entity>::new();
        let visible_mesh_entities: HashSet<Entity> = camera_visible_meshes
            .single()
            .map(|visible_entities| {
                visible_entities
                    .iter(TypeId::of::<Mesh3d>())
                    .copied()
                    .collect()
            })
            .unwrap_or_default();
        let mut mesh_count = 0usize;
        let mut visible_mesh_count = 0usize;
        let mut polygon_count = 0u64;
        let mut visible_polygon_count = 0u64;

        while let Some(entity) = stack.pop() {
            if !visited_entities.insert(entity) {
                continue;
            }

            if let Ok(mesh_handle) = mesh_handles.get(entity) {
                mesh_count += 1;
                let triangle_count = if let Some(mesh) = meshes.get(&mesh_handle.0) {
                    let mesh_id = mesh_handle.0.id();
                    *mesh_triangles_by_id
                        .entry(mesh_id)
                        .or_insert_with(|| triangles_for_mesh(mesh))
                } else {
                    0
                };
                polygon_count = polygon_count.saturating_add(triangle_count);

                if visible_mesh_entities.contains(&entity) {
                    visible_mesh_count += 1;
                    visible_polygon_count = visible_polygon_count.saturating_add(triangle_count);
                    if let Some(scene_object_root) =
                        find_scene_object_ancestor(entity, &parent_query, &scene_objects)
                    {
                        visible_objects.insert(scene_object_root);
                    }
                }
            }

            if let Ok(children) = children_query.get(entity) {
                for child in children.iter() {
                    stack.push(child);
                }
            }
        }

        debug_stats.object_count = scene_objects.iter().count();
        debug_stats.visible_object_count = visible_objects.len();
        debug_stats.mesh_count = mesh_count;
        debug_stats.visible_mesh_count = visible_mesh_count;
        debug_stats.polygon_count = polygon_count;
        debug_stats.visible_polygon_count = visible_polygon_count;
        debug_stats.last_runtime_entity_count = runtime_entity_count;
        debug_stats.last_mesh_asset_count = mesh_asset_count;
    }

    let fps = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|diagnostic| diagnostic.smoothed().or_else(|| diagnostic.value()));
    let frame_time_ms = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FRAME_TIME)
        .and_then(|diagnostic| diagnostic.smoothed().or_else(|| diagnostic.value()));

    let fps_text = fps
        .map(|value| format!("{value:.1}"))
        .unwrap_or_else(|| "n/a".to_string());
    let frame_text = frame_time_ms
        .map(|value| format!("{value:.2}"))
        .unwrap_or_else(|| "n/a".to_string());
    let (gpu_name, graphics_api, graphics_version) = if let Some(info) = adapter_info {
        let api = format!("{:?}", info.backend);
        let gpu = sanitize_info_text(&info.name);
        let driver = sanitize_info_text(&info.driver);
        let driver_info = sanitize_info_text(&info.driver_info);
        let version = if driver_info == "n/a" {
            driver
        } else {
            format!("{driver} {driver_info}")
        };
        (gpu, api, version)
    } else {
        ("n/a".to_string(), "n/a".to_string(), "n/a".to_string())
    };

    let elapsed_text = format!("{:.1}", time.elapsed_secs());
    let distance_culling_text = if let Some(config) = distance_culling {
        if config.enabled {
            format!("{:.0}", config.max_distance)
        } else {
            "off".to_string()
        }
    } else {
        "n/a".to_string()
    };
    for mut text in &mut perf_text_query {
        text.0 = if cfg!(debug_assertions) {
            format!(
                "FPS: {fps_text}\nFrame: {frame_text} ms\nTempo: {elapsed_text} s\nCull dist: {distance_culling_text}\nObjetos render: {}/{}\nMeshes render: {}/{}\nPoligonos render: {}/{}",
                debug_stats.visible_object_count,
                debug_stats.object_count,
                debug_stats.visible_mesh_count,
                debug_stats.mesh_count,
                debug_stats.visible_polygon_count,
                debug_stats.polygon_count,
            )
        } else {
            format!("FPS: {fps_text}\nFrame: {frame_text} ms\nTempo: {elapsed_text} s")
        };
    }

    let frame_limit_text = debug_extras
        .frame_limiter
        .as_ref()
        .map(|fl| format!("{}", fl.mode))
        .unwrap_or_else(|| "n/a".to_string());
    let shadow_text = debug_extras
        .shadow_quality
        .as_ref()
        .map(|sq| format!("{}", sq.mode))
        .unwrap_or_else(|| "n/a".to_string());

    for mut text in &mut gpu_text_query {
        text.0 = format!(
            "GPU: {gpu_name}\nVideo API: {graphics_api}\nVersao: {graphics_version}\n[F4] FPS Limit: {frame_limit_text}\n[F5] Sombra: {shadow_text}",
        );
    }
}

fn triangles_for_mesh(mesh: &Mesh) -> u64 {
    let primitive_count = match mesh.indices() {
        Some(Indices::U16(indices)) => indices.len(),
        Some(Indices::U32(indices)) => indices.len(),
        None => mesh.count_vertices(),
    };

    match mesh.primitive_topology() {
        PrimitiveTopology::TriangleList => (primitive_count / 3) as u64,
        PrimitiveTopology::TriangleStrip => primitive_count.saturating_sub(2) as u64,
        _ => 0,
    }
}

fn sanitize_info_text(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        "n/a".to_string()
    } else {
        trimmed.to_string()
    }
}

fn find_scene_object_ancestor(
    start: Entity,
    parents: &Query<&ChildOf>,
    scene_objects: &Query<(), With<SceneObject>>,
) -> Option<Entity> {
    if scene_objects.contains(start) {
        return Some(start);
    }

    let mut current = start;
    while let Ok(parent) = parents.get(current) {
        let parent_entity = parent.parent();
        if scene_objects.contains(parent_entity) {
            return Some(parent_entity);
        }
        current = parent_entity;
    }

    None
}
