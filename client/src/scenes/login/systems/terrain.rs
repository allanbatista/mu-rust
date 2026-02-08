use crate::scenes::login::components::*;
use crate::scenes::login::LoginSceneAssets;
use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};

/// Marker component to track if terrain has been spawned
#[derive(Component)]
pub struct TerrainSpawned;

/// System to spawn terrain once assets are loaded
pub fn spawn_terrain_when_ready(
    mut commands: Commands,
    assets: Res<LoginSceneAssets>,
    terrain_configs: Res<Assets<TerrainConfig>>,
    heightmaps: Res<Assets<HeightmapData>>,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<TerrainMaterial>>,
    terrain_query: Query<&TerrainSpawned>,
) {
    // Only spawn once
    if !terrain_query.is_empty() {
        return;
    }

    // Wait for assets to be loaded
    if !assets.loaded {
        return;
    }

    let Some(config) = terrain_configs.get(&assets.terrain_config) else {
        return;
    };

    let Some(heightmap) = heightmaps.get(&assets.heightmap) else {
        return;
    };

    info!("Spawning terrain mesh ({}x{})", heightmap.width, heightmap.height);

    // Generate terrain mesh
    let mesh = generate_terrain_mesh(heightmap, config);
    let mesh_handle = meshes.add(mesh);

    info!("Terrain mesh generated successfully");

    // Create simple material (no textures for now)
    let material = TerrainMaterial {};
    let material_handle = materials.add(material);

    // Spawn terrain entity centered
    let entity = commands.spawn((
        TerrainSpawned,
        Terrain {
            width: config.size.width,
            height: config.size.depth,
        },
        MaterialMeshBundle {
            mesh: mesh_handle,
            material: material_handle,
            transform: Transform::from_xyz(0.0, 0.0, 0.0),
            ..default()
        },
    )).id();

    info!("Terrain spawned successfully at entity {:?}", entity);
}

/// Generate a terrain mesh from heightmap data
fn generate_terrain_mesh(heightmap: &HeightmapData, config: &TerrainConfig) -> Mesh {
    let width = heightmap.width as usize;
    let height = heightmap.height as usize;
    let scale = config.size.scale;

    info!("Generating mesh: {}x{} vertices, scale={}", width, height, scale);

    let mut positions = Vec::new();
    let mut uvs = Vec::new();
    let mut indices = Vec::new();

    // Generate vertices
    for z in 0..height {
        for x in 0..width {
            let h = heightmap.get_height(x, z) * 50.0; // Scale height for visibility
            positions.push([x as f32 * scale, h, z as f32 * scale]);
            uvs.push([x as f32 / width as f32, z as f32 / height as f32]);
        }
    }

    info!("Generated {} vertices", positions.len());

    // Generate indices for triangles
    for z in 0..(height - 1) {
        for x in 0..(width - 1) {
            let top_left = (z * width + x) as u32;
            let top_right = top_left + 1;
            let bottom_left = ((z + 1) * width + x) as u32;
            let bottom_right = bottom_left + 1;

            // First triangle
            indices.push(top_left);
            indices.push(bottom_left);
            indices.push(top_right);

            // Second triangle
            indices.push(top_right);
            indices.push(bottom_left);
            indices.push(bottom_right);
        }
    }

    // Calculate normals
    let normals = calculate_normals(&positions, &indices);

    info!("Generated {} triangles", indices.len() / 3);

    Mesh::new(PrimitiveTopology::TriangleList, default())
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
        .with_inserted_indices(Indices::U32(indices))
}

/// Calculate vertex normals from positions and indices
fn calculate_normals(positions: &[[f32; 3]], indices: &[u32]) -> Vec<[f32; 3]> {
    let mut normals = vec![[0.0f32, 0.0, 0.0]; positions.len()];

    // Calculate face normals and accumulate
    for triangle in indices.chunks(3) {
        let i0 = triangle[0] as usize;
        let i1 = triangle[1] as usize;
        let i2 = triangle[2] as usize;

        let p0 = Vec3::from(positions[i0]);
        let p1 = Vec3::from(positions[i1]);
        let p2 = Vec3::from(positions[i2]);

        let edge1 = p1 - p0;
        let edge2 = p2 - p0;
        let normal = edge1.cross(edge2).normalize();

        // Accumulate normal for each vertex
        normals[i0][0] += normal.x;
        normals[i0][1] += normal.y;
        normals[i0][2] += normal.z;

        normals[i1][0] += normal.x;
        normals[i1][1] += normal.y;
        normals[i1][2] += normal.z;

        normals[i2][0] += normal.x;
        normals[i2][1] += normal.y;
        normals[i2][2] += normal.z;
    }

    // Normalize accumulated normals
    for normal in &mut normals {
        let n = Vec3::from(*normal).normalize();
        *normal = [n.x, n.y, n.z];
    }

    normals
}
