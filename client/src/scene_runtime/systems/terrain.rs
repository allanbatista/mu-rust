use crate::scene_runtime::components::*;
use crate::scene_runtime::state::RuntimeSceneAssets;
use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy::render::texture::{
    ImageAddressMode, ImageLoaderSettings, ImageSampler, ImageSamplerDescriptor,
};
use std::fs;
use std::path::{Path, PathBuf};

const TERRAIN_TILE_UV_STEP: f32 = 0.25;
const CLIENT_ASSETS_ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../assets");

/// Marker component to track if terrain has been spawned
#[derive(Component)]
pub struct TerrainSpawned;

/// System to spawn terrain once assets are loaded
pub fn spawn_terrain_when_ready(
    mut commands: Commands,
    assets: Res<RuntimeSceneAssets>,
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

    let diffuse_path = resolve_terrain_diffuse_path(config, &world.world_name);
    info!(
        "Using terrain base texture '{}' (lightmap='{}')",
        diffuse_path, config.lightmap
    );
    let diffuse = asset_server.load_with_settings(diffuse_path, |settings: &mut _| {
        *settings = ImageLoaderSettings {
            sampler: ImageSampler::Descriptor(ImageSamplerDescriptor {
                address_mode_u: ImageAddressMode::Repeat,
                address_mode_v: ImageAddressMode::Repeat,
                ..default()
            }),
            ..default()
        };
    });
    let material_handle = materials.add(StandardMaterial {
        base_color_texture: Some(diffuse),
        perceptual_roughness: 1.0,
        metallic: 0.0,
        reflectance: 0.0,
        ..default()
    });

    // Spawn terrain entity centered
    let entity = commands
        .spawn((
            RuntimeSceneEntity,
            TerrainSpawned,
            Terrain,
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
    let layer_uv_scale = config
        .texture_layers
        .first()
        .map(|layer| layer.scale.max(0.01))
        .unwrap_or(1.0);
    let uv_step = TERRAIN_TILE_UV_STEP / layer_uv_scale;

    // Generate vertices
    for z in 0..height {
        for x in 0..width {
            let h = heightmap.get_height(x, z) * vertical_scale;
            min_height = min_height.min(h);
            max_height = max_height.max(h);
            positions.push([x as f32 * scale, h, z as f32 * scale]);
            let u = x as f32 * uv_step;
            let v = z as f32 * uv_step;
            uvs.push([u, v]);
        }
    }

    info!("Generated {} vertices", positions.len());
    info!(
        "Terrain UV tiling step set to {:.4} (layer scale {:.3})",
        uv_step, layer_uv_scale
    );
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

fn resolve_terrain_diffuse_path(config: &TerrainConfig, world_name: &str) -> String {
    let mut candidates: Vec<String> = config
        .texture_layers
        .iter()
        .map(|layer| layer.path.trim().to_string())
        .filter(|path| !path.is_empty())
        .collect();

    candidates.push(format!("data/{world_name}/TileGrass01.png"));
    candidates.push(format!("data/{world_name}/tilegrass01.png"));

    for candidate in &candidates {
        if let Some(resolved) = resolve_existing_asset_path(candidate) {
            return resolved;
        }
    }

    if let Some(lightmap) = resolve_existing_asset_path(&config.lightmap) {
        warn!(
            "Terrain layer textures not found for world '{}'; falling back to lightmap '{}'",
            world_name, lightmap
        );
        return lightmap;
    }

    candidates
        .first()
        .cloned()
        .unwrap_or_else(|| format!("data/{world_name}/TileGrass01.png"))
}

fn resolve_existing_asset_path(raw_path: &str) -> Option<String> {
    let normalized = normalize_asset_path(raw_path);
    if normalized.is_empty() {
        return None;
    }

    let root = Path::new(CLIENT_ASSETS_ROOT);
    let full = root.join(&normalized);
    if full.is_file() {
        return Some(normalized);
    }

    resolve_case_insensitive_path(root, &normalized)
}

fn normalize_asset_path(raw_path: &str) -> String {
    raw_path
        .trim()
        .replace('\\', "/")
        .trim_start_matches('/')
        .to_string()
}

fn resolve_case_insensitive_path(root: &Path, rel_path: &str) -> Option<String> {
    let mut current_dir = PathBuf::from(root);
    let mut resolved_parts: Vec<String> = Vec::new();

    for part in rel_path.split('/').filter(|segment| !segment.is_empty()) {
        let direct = current_dir.join(part);
        if direct.exists() {
            current_dir = direct;
            resolved_parts.push(part.to_string());
            continue;
        }

        let needle = part.to_lowercase();
        let mut matched_name: Option<String> = None;

        for entry in fs::read_dir(&current_dir).ok()? {
            let entry = entry.ok()?;
            let name = entry.file_name().to_string_lossy().to_string();
            if name.to_lowercase() == needle {
                matched_name = Some(name);
                current_dir = entry.path();
                break;
            }
        }

        let matched = matched_name?;
        resolved_parts.push(matched);
    }

    if current_dir.is_file() {
        Some(resolved_parts.join("/"))
    } else {
        None
    }
}
