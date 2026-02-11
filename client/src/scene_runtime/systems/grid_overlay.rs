use crate::bevy_compat::*;
use crate::grid_overlay::{
    GRID_OVERLAY_COLOR, GridOverlayConfig, build_grid_segments, grid_line_count, segment_transform,
};
use crate::scene_runtime::components::RuntimeSceneEntity;
use crate::scene_runtime::scene_loader::{HeightmapData, TerrainConfig};
use crate::scene_runtime::state::RuntimeSceneAssets;
use crate::scene_runtime::world_coordinates::{
    WorldMirrorAxis, mirror_map_xz_with_axis, world_mirror_axis,
};
use bevy::light::{NotShadowCaster, NotShadowReceiver};
use bevy::prelude::*;

const GRID_VISIBLE_HALF_CELLS: i32 = 25;
const GRID_Y_OFFSET: f32 = 1.0;
const GRID_LINE_THICKNESS: f32 = 1.0;

#[derive(Component)]
pub(crate) struct RuntimeMapGridLine {
    index: usize,
}

fn effective_height_multiplier(world_name: &str, config: &TerrainConfig) -> f32 {
    if world_name == "world_56" {
        3.0
    } else {
        config.height_multiplier
    }
}

fn terrain_height_at_world(
    heightmap: &HeightmapData,
    world_x: f32,
    world_z: f32,
    cell_size: f32,
    vertical_scale: f32,
    map_max_x: f32,
    map_max_z: f32,
    mirror_axis: WorldMirrorAxis,
) -> f32 {
    let width = heightmap.width as usize;
    let depth = heightmap.height as usize;
    if width == 0 || depth == 0 || cell_size <= 0.0 {
        return 0.0;
    }

    let (map_world_x, map_world_z) =
        mirror_map_xz_with_axis(world_x, world_z, map_max_x, map_max_z, mirror_axis);

    let grid_x = (map_world_x / cell_size).clamp(0.0, width.saturating_sub(1) as f32);
    let grid_z = (map_world_z / cell_size).clamp(0.0, depth.saturating_sub(1) as f32);

    let x0 = grid_x.floor() as usize;
    let z0 = grid_z.floor() as usize;
    let x1 = (x0 + 1).min(width.saturating_sub(1));
    let z1 = (z0 + 1).min(depth.saturating_sub(1));

    let tx = grid_x - x0 as f32;
    let tz = grid_z - z0 as f32;

    let h00 = heightmap.get_height(x0, z0) * vertical_scale;
    let h10 = heightmap.get_height(x1, z0) * vertical_scale;
    let h01 = heightmap.get_height(x0, z1) * vertical_scale;
    let h11 = heightmap.get_height(x1, z1) * vertical_scale;

    let h0 = h00 + (h10 - h00) * tx;
    let h1 = h01 + (h11 - h01) * tx;
    h0 + (h1 - h0) * tz
}

fn spawn_runtime_map_grid_lines(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    let line_mesh = meshes.add(Mesh::from(Cuboid::new(1.0, 1.0, 1.0)));
    let line_material = materials.add(StandardMaterial {
        base_color: GRID_OVERLAY_COLOR,
        emissive: LinearRgba::rgb(1.0, 1.0, 1.0),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        cull_mode: None,
        ..default()
    });

    for index in 0..grid_line_count(GRID_VISIBLE_HALF_CELLS) {
        commands.spawn((
            RuntimeSceneEntity,
            RuntimeMapGridLine { index },
            NotShadowCaster,
            NotShadowReceiver,
            PbrBundle {
                mesh: Mesh3d(line_mesh.clone()),
                material: MeshMaterial3d(line_material.clone()),
                transform: Transform::from_scale(Vec3::splat(0.001)),
                ..default()
            },
        ));
    }
}

pub fn draw_runtime_map_grid(
    mut commands: Commands,
    runtime_assets: Option<Res<RuntimeSceneAssets>>,
    terrain_configs: Res<Assets<TerrainConfig>>,
    heightmaps: Res<Assets<HeightmapData>>,
    camera_query: Query<&Transform, (With<Camera3d>, Without<RuntimeMapGridLine>)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    existing_lines: Query<Entity, With<RuntimeMapGridLine>>,
    mut line_transforms: Query<(&RuntimeMapGridLine, &mut Transform), Without<Camera3d>>,
) {
    if existing_lines.is_empty() {
        spawn_runtime_map_grid_lines(&mut commands, &mut meshes, &mut materials);
    }

    let Some(runtime_assets) = runtime_assets else {
        return;
    };
    if !runtime_assets.loaded {
        return;
    }

    let Some(world) = runtime_assets.world.as_ref() else {
        return;
    };

    let Some(terrain_config) = terrain_configs.get(&world.terrain_config) else {
        return;
    };

    let Some(heightmap) = heightmaps.get(&world.heightmap) else {
        return;
    };

    if terrain_config.size.scale <= 0.0 || heightmap.width < 2 || heightmap.height < 2 {
        return;
    }

    let map_max_x = (heightmap.width.saturating_sub(1) as f32) * terrain_config.size.scale;
    let map_max_z = (heightmap.height.saturating_sub(1) as f32) * terrain_config.size.scale;
    let mirror_axis = world_mirror_axis();
    let vertical_scale = effective_height_multiplier(&world.world_name, terrain_config)
        * (terrain_config.size.scale / terrain_config.legacy_terrain_scale.max(1.0));

    let center = camera_query
        .iter()
        .next()
        .map(|transform| transform.translation)
        .unwrap_or(Vec3::new(map_max_x * 0.5, 0.0, map_max_z * 0.5));

    let segments = build_grid_segments(
        center,
        GridOverlayConfig {
            cell_size: terrain_config.size.scale,
            visible_half_cells: GRID_VISIBLE_HALF_CELLS,
            y_offset: GRID_Y_OFFSET,
            color: GRID_OVERLAY_COLOR,
        },
        |world_x, world_z| {
            terrain_height_at_world(
                heightmap,
                world_x,
                world_z,
                terrain_config.size.scale,
                vertical_scale,
                map_max_x,
                map_max_z,
                mirror_axis,
            )
        },
    );

    for (line, mut transform) in &mut line_transforms {
        if let Some(segment) = segments.get(line.index).copied() {
            if let Some(next_transform) = segment_transform(segment, GRID_LINE_THICKNESS) {
                *transform = next_transform;
                continue;
            }
        }
        transform.scale = Vec3::splat(0.001);
    }
}
