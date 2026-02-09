use crate::scene_runtime::components::*;
use crate::scene_runtime::state::RuntimeSceneAssets;
use crate::scene_runtime::systems::terrain::TerrainSpawned;
use bevy::prelude::*;
use bevy::render::mesh::PrimitiveTopology;
use bevy::render::render_resource::Face;

/// Height of the boundary walls in world units.
const WALL_HEIGHT: f32 = 3000.0;

/// How far below the terrain minimum to extend the walls.
const WALL_DEPTH_BELOW: f32 = 500.0;

/// Marker component to track if boundary walls have been spawned.
#[derive(Component)]
pub struct BoundaryWallsSpawned;

/// System to spawn black boundary walls around the map edges once terrain is ready.
pub fn spawn_boundary_walls_when_ready(
    mut commands: Commands,
    assets: Res<RuntimeSceneAssets>,
    terrain_configs: Res<Assets<TerrainConfig>>,
    heightmaps: Res<Assets<HeightmapData>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    terrain_query: Query<&TerrainSpawned>,
    walls_query: Query<&BoundaryWallsSpawned>,
) {
    // Only spawn after terrain exists and walls haven't been created yet.
    if terrain_query.is_empty() || !walls_query.is_empty() || !assets.loaded {
        return;
    }

    let Some(world) = assets.world.as_ref() else {
        return;
    };

    let Some(config) = terrain_configs.get(&world.terrain_config) else {
        return;
    };

    let Some(heightmap) = heightmaps.get(&world.heightmap) else {
        return;
    };

    let width = heightmap.width as usize;
    let depth = heightmap.height as usize;
    let scale = config.size.scale;
    let max_x = (width.saturating_sub(1) as f32) * scale;
    let max_z = (depth.saturating_sub(1) as f32) * scale;
    let vertical_scale =
        config.height_multiplier * (scale / config.legacy_terrain_scale.max(1.0));

    // Scan heightmap edges to find reasonable base heights per wall.
    let mut min_h = f32::MAX;
    for z in 0..depth {
        for x in 0..width {
            let h = heightmap.get_height(x, z) * vertical_scale;
            min_h = min_h.min(h);
        }
    }
    if min_h == f32::MAX {
        min_h = 0.0;
    }

    let bottom = min_h - WALL_DEPTH_BELOW;
    let top = min_h + WALL_HEIGHT;

    // Flat black, unlit material – no texture needed.
    let wall_material = materials.add(StandardMaterial {
        base_color: Color::BLACK,
        unlit: true,
        cull_mode: Some(Face::Back),
        ..default()
    });

    let root = commands
        .spawn((
            RuntimeSceneEntity,
            BoundaryWallsSpawned,
            SpatialBundle::default(),
        ))
        .id();

    // Wall definitions: (corner_a, corner_b) as (x, z) pairs – the wall extends
    // vertically between `bottom` and `top`.
    //
    //  (0,max_z) ---- (max_x,max_z)
    //      |                |
    //      |     terrain    |
    //      |                |
    //  (0,0) ---------- (max_x,0)
    //
    let walls: [(Vec3, Vec3, Vec3, Vec3); 4] = [
        // South wall (z = 0), facing +Z (inward)
        (
            Vec3::new(0.0, bottom, 0.0),
            Vec3::new(max_x, bottom, 0.0),
            Vec3::new(max_x, top, 0.0),
            Vec3::new(0.0, top, 0.0),
        ),
        // North wall (z = max_z), facing -Z (inward)
        (
            Vec3::new(max_x, bottom, max_z),
            Vec3::new(0.0, bottom, max_z),
            Vec3::new(0.0, top, max_z),
            Vec3::new(max_x, top, max_z),
        ),
        // West wall (x = 0), facing +X (inward)
        (
            Vec3::new(0.0, bottom, max_z),
            Vec3::new(0.0, bottom, 0.0),
            Vec3::new(0.0, top, 0.0),
            Vec3::new(0.0, top, max_z),
        ),
        // East wall (x = max_x), facing -X (inward)
        (
            Vec3::new(max_x, bottom, 0.0),
            Vec3::new(max_x, bottom, max_z),
            Vec3::new(max_x, top, max_z),
            Vec3::new(max_x, top, 0.0),
        ),
    ];

    for (a, b, c, d) in &walls {
        let mesh = build_quad_mesh(*a, *b, *c, *d);
        let mesh_handle = meshes.add(mesh);

        commands.entity(root).with_children(|parent| {
            parent.spawn((
                RuntimeSceneEntity,
                PbrBundle {
                    mesh: mesh_handle,
                    material: wall_material.clone(),
                    transform: Transform::IDENTITY,
                    ..default()
                },
            ));
        });
    }

    info!(
        "Boundary walls spawned for '{}': map {}x{} (max_x={:.0}, max_z={:.0}), bottom={:.0}, top={:.0}",
        world.world_name, width, depth, max_x, max_z, bottom, top
    );
}

/// Build a single-quad mesh from four corners (counter-clockwise winding for front face).
fn build_quad_mesh(a: Vec3, b: Vec3, c: Vec3, d: Vec3) -> Mesh {
    let edge1 = b - a;
    let edge2 = d - a;
    let normal = edge1.cross(edge2).normalize_or_zero();
    let n = [normal.x, normal.y, normal.z];

    let positions = vec![
        [a.x, a.y, a.z],
        [b.x, b.y, b.z],
        [c.x, c.y, c.z],
        [d.x, d.y, d.z],
    ];
    let normals = vec![n, n, n, n];
    let uvs: Vec<[f32; 2]> = vec![[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]];
    let indices = vec![0u32, 1, 2, 0, 2, 3];

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(bevy::render::mesh::Indices::U32(indices));
    mesh
}
