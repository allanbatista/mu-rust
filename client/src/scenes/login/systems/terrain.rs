use crate::scenes::login::LoginSceneAssets;
use crate::scenes::login::components::*;
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
    mut materials: ResMut<Assets<StandardMaterial>>,
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

    let Some(world) = assets.world.as_ref() else {
        return;
    };

    let Some(config) = terrain_configs.get(&world.terrain_config) else {
        return;
    };

    let Some(heightmap) = heightmaps.get(&world.heightmap) else {
        return;
    };

    info!(
        "Spawning terrain mesh ({}x{})",
        heightmap.width, heightmap.height
    );

    // Generate terrain mesh
    let mesh = generate_terrain_mesh(heightmap, config);
    let mesh_handle = meshes.add(mesh);

    info!("Terrain mesh generated successfully");

    // Prefer the pre-baked terrain lightmap to avoid stretched single-tile rendering.
    let diffuse_path = if config.lightmap.trim().is_empty() {
        config
            .texture_layers
            .first()
            .map(|layer| layer.path.clone())
            .unwrap_or_else(|| "data/World74/TileGrass01.png".to_string())
    } else {
        config.lightmap.clone()
    };
    info!("Using terrain diffuse texture '{}'", diffuse_path);
    let diffuse = asset_server.load(diffuse_path);
    let material_handle = materials.add(StandardMaterial {
        base_color_texture: Some(diffuse),
        perceptual_roughness: 1.0,
        metallic: 0.0,
        ..default()
    });

    // Spawn terrain entity centered
    let entity = commands
        .spawn((
            LoginSceneEntity,
            TerrainSpawned,
            Terrain {
                width: config.size.width,
                height: config.size.depth,
            },
            PbrBundle {
                mesh: mesh_handle,
                material: material_handle,
                transform: Transform::from_xyz(0.0, 0.0, 0.0),
                ..default()
            },
        ))
        .id();

    info!("Terrain spawned successfully at entity {:?}", entity);
}

/// Generate a terrain mesh from heightmap data
fn generate_terrain_mesh(heightmap: &HeightmapData, config: &TerrainConfig) -> Mesh {
    let width = heightmap.width as usize;
    let height = heightmap.height as usize;
    let scale = config.size.scale;
    let vertical_scale = config.height_multiplier * (scale / config.legacy_terrain_scale.max(1.0));

    info!(
        "Generating mesh: {}x{} vertices, scale={}, vertical_scale={}",
        width, height, scale, vertical_scale
    );

    let mut positions = Vec::new();
    let mut uvs = Vec::new();
    let mut indices = Vec::new();
    let mut min_height = f32::MAX;
    let mut max_height = f32::MIN;

    // Generate vertices
    for z in 0..height {
        for x in 0..width {
            let h = heightmap.get_height(x, z) * vertical_scale;
            min_height = min_height.min(h);
            max_height = max_height.max(h);
            positions.push([x as f32 * scale, h, z as f32 * scale]);
            let u = if width > 1 {
                x as f32 / (width - 1) as f32
            } else {
                0.0
            };
            let v = if height > 1 {
                z as f32 / (height - 1) as f32
            } else {
                0.0
            };
            uvs.push([u, v]);
        }
    }

    info!("Generated {} vertices", positions.len());
    info!(
        "Terrain height range: min={:.3}, max={:.3}",
        min_height, max_height
    );
    if (max_height - min_height).abs() <= f32::EPSILON {
        warn!(
            "Terrain heightmap is flat; this usually indicates the source world relies on static object geometry"
        );
    }

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
