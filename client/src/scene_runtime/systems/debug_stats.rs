use crate::scene_runtime::components::*;
use bevy::asset::AssetId;
use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy::render::renderer::RenderAdapterInfo;
use std::collections::{HashMap, HashSet};

/// UI marker for scene stats text shown in debug mode.
#[derive(Component)]
pub struct DebugSceneStatsText;

#[derive(Resource)]
pub struct DebugSceneStats {
    pub refresh_timer: Timer,
    pub object_count: usize,
    pub mesh_count: usize,
    pub polygon_count: u64,
    pub last_runtime_entity_count: usize,
    pub last_mesh_asset_count: usize,
}

impl Default for DebugSceneStats {
    fn default() -> Self {
        Self {
            refresh_timer: Timer::from_seconds(0.75, TimerMode::Repeating),
            object_count: 0,
            mesh_count: 0,
            polygon_count: 0,
            last_runtime_entity_count: 0,
            last_mesh_asset_count: 0,
        }
    }
}

pub fn reset_debug_scene_stats(mut debug_stats: ResMut<DebugSceneStats>) {
    *debug_stats = DebugSceneStats::default();
}

pub fn spawn_debug_scene_stats_hud(mut commands: Commands) {
    commands.spawn((
        RuntimeSceneEntity,
        DebugSceneStatsText,
        TextBundle::from_section(
            "",
            TextStyle {
                font_size: 16.0,
                color: Color::srgb(0.85, 0.95, 0.95),
                ..default()
            },
        )
        .with_style(Style {
            position_type: PositionType::Absolute,
            top: Val::Px(38.0),
            left: Val::Px(14.0),
            ..default()
        }),
    ));
}

pub fn update_debug_scene_stats(
    time: Res<Time>,
    diagnostics: Res<DiagnosticsStore>,
    adapter_info: Option<Res<RenderAdapterInfo>>,
    meshes: Res<Assets<Mesh>>,
    login_entities: Query<Entity, With<RuntimeSceneEntity>>,
    scene_objects: Query<(), With<SceneObject>>,
    children_query: Query<&Children>,
    mesh_handles: Query<&Handle<Mesh>>,
    mut debug_stats: ResMut<DebugSceneStats>,
    mut text_query: Query<&mut Text, With<DebugSceneStatsText>>,
) {
    debug_stats.refresh_timer.tick(time.delta());
    if debug_stats.refresh_timer.just_finished() {
        let runtime_entity_count = login_entities.iter().count();
        let mesh_asset_count = meshes.len();

        if runtime_entity_count != debug_stats.last_runtime_entity_count
            || mesh_asset_count != debug_stats.last_mesh_asset_count
        {
            let mut visited_entities = HashSet::<Entity>::new();
            let mut stack: Vec<Entity> = login_entities.iter().collect();
            let mut mesh_triangles_by_id = HashMap::<AssetId<Mesh>, u64>::new();
            let mut mesh_count = 0usize;
            let mut polygon_count = 0u64;

            while let Some(entity) = stack.pop() {
                if !visited_entities.insert(entity) {
                    continue;
                }

                if let Ok(mesh_handle) = mesh_handles.get(entity) {
                    mesh_count += 1;
                    if let Some(mesh) = meshes.get(mesh_handle) {
                        let mesh_id = mesh_handle.id();
                        let triangle_count = *mesh_triangles_by_id
                            .entry(mesh_id)
                            .or_insert_with(|| triangles_for_mesh(mesh));
                        polygon_count = polygon_count.saturating_add(triangle_count);
                    }
                }

                if let Ok(children) = children_query.get(entity) {
                    for child in children.iter() {
                        stack.push(*child);
                    }
                }
            }

            debug_stats.object_count = scene_objects.iter().count();
            debug_stats.mesh_count = mesh_count;
            debug_stats.polygon_count = polygon_count;
            debug_stats.last_runtime_entity_count = runtime_entity_count;
            debug_stats.last_mesh_asset_count = mesh_asset_count;
        }
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

    for mut text in &mut text_query {
        text.sections[0].value = format!(
            "[DEBUG] FPS: {fps_text} | Frame: {frame_text} ms | Objetos: {} | Meshes: {} | Poligonos: {}\n[DEBUG] GPU: {gpu_name} | API: {graphics_api} | Versao: {graphics_version}",
            debug_stats.object_count, debug_stats.mesh_count, debug_stats.polygon_count,
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
