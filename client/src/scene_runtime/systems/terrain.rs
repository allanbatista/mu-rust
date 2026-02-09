use super::grass::find_grass_slots;
use crate::scene_runtime::components::*;
use crate::scene_runtime::state::RuntimeSceneAssets;
use bevy::prelude::*;
use bevy::render::mesh::PrimitiveTopology;
use bevy::render::render_resource::Face;
use bevy::render::texture::{
    ImageAddressMode, ImageLoaderSettings, ImageSampler, ImageSamplerDescriptor,
};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

const TERRAIN_TILE_UV_STEP: f32 = 0.25;
const CLIENT_ASSETS_ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../assets");
const TERRAIN_NO_LAYER_SLOT: u8 = 255;

/// Marker component to track if terrain has been spawned.
#[derive(Component)]
pub struct TerrainSpawned;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
enum TerrainPass {
    Base,
    Alpha,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
struct TerrainBatchKey {
    texture_slot: u8,
    pass: TerrainPass,
}

#[derive(Default)]
struct TerrainBatch {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    colors: Vec<[f32; 4]>,
    uvs: Vec<[f32; 2]>,
}

impl TerrainBatch {
    fn triangles(&self) -> usize {
        self.positions.len() / 3
    }
}

struct PreparedTerrainBatch {
    mesh: Mesh,
    texture_path: String,
    texture_slot: u8,
    pass: TerrainPass,
    triangles: usize,
}

/// System to spawn terrain once all scene assets are ready.
pub fn spawn_terrain_when_ready(
    mut commands: Commands,
    assets: Res<RuntimeSceneAssets>,
    terrain_configs: Res<Assets<TerrainConfig>>,
    heightmaps: Res<Assets<HeightmapData>>,
    terrain_maps: Res<Assets<TerrainMapData>>,
    terrain_texture_slots: Res<Assets<TerrainTextureSlotsData>>,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    terrain_query: Query<&TerrainSpawned>,
) {
    if !terrain_query.is_empty() || !assets.loaded {
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

    let terrain_map = terrain_maps.get(&world.terrain_map).or_else(|| {
        world
            .legacy_terrain_map
            .as_ref()
            .and_then(|fallback| terrain_maps.get(fallback))
    });
    let Some(terrain_map) = terrain_map else {
        return;
    };

    let texture_slots_data = world
        .terrain_texture_slots
        .as_ref()
        .and_then(|handle| terrain_texture_slots.get(handle));
    let grass_slots = find_grass_slots(texture_slots_data, &world.world_name);

    let prepared_batches = build_terrain_batches(
        heightmap,
        config,
        terrain_map,
        texture_slots_data,
        &world.world_name,
        if grass_slots.is_empty() {
            None
        } else {
            Some(&grass_slots)
        },
    );

    if prepared_batches.is_empty() {
        warn!(
            "No terrain render batches were generated for '{}'",
            world.world_name
        );
        return;
    }

    let mut material_cache: HashMap<(String, TerrainPass), Handle<StandardMaterial>> =
        HashMap::new();

    let root = commands
        .spawn((
            RuntimeSceneEntity,
            TerrainSpawned,
            Terrain,
            SpatialBundle::default(),
        ))
        .id();

    let mut total_triangles = 0usize;
    let mut total_tiles = 0usize;
    let mut spawned_batches = 0usize;

    for batch in prepared_batches {
        if batch.triangles == 0 {
            continue;
        }

        let mesh_handle = meshes.add(batch.mesh);
        let material_key = (batch.texture_path.clone(), batch.pass);

        let material_handle = if let Some(existing) = material_cache.get(&material_key) {
            existing.clone()
        } else {
            let diffuse =
                asset_server.load_with_settings(batch.texture_path.clone(), |settings: &mut _| {
                    *settings = ImageLoaderSettings {
                        // Terrain diffuse/albedo textures are authored in sRGB color space.
                        is_srgb: true,
                        sampler: ImageSampler::Descriptor(ImageSamplerDescriptor {
                            address_mode_u: ImageAddressMode::Repeat,
                            address_mode_v: ImageAddressMode::Repeat,
                            ..default()
                        }),
                        ..default()
                    };
                });

            let handle = materials.add(StandardMaterial {
                base_color_texture: Some(diffuse),
                alpha_mode: match batch.pass {
                    TerrainPass::Base => AlphaMode::Opaque,
                    TerrainPass::Alpha => AlphaMode::Blend,
                },
                perceptual_roughness: 1.0,
                metallic: 0.0,
                reflectance: 0.0,
                unlit: false,
                cull_mode: Some(Face::Back),
                ..default()
            });

            material_cache.insert(material_key, handle.clone());
            handle
        };

        total_triangles += batch.triangles;
        total_tiles += batch.triangles / 2;
        spawned_batches += 1;

        commands.entity(root).with_children(|parent| {
            parent.spawn((
                RuntimeSceneEntity,
                Terrain,
                PbrBundle {
                    mesh: mesh_handle,
                    material: material_handle,
                    transform: Transform::IDENTITY,
                    ..default()
                },
            ));
        });
    }

    info!(
        "Terrain '{}' spawned with {} batches / {} tiles / {} triangles",
        world.world_name, spawned_batches, total_tiles, total_triangles
    );
}

fn build_terrain_batches(
    heightmap: &HeightmapData,
    config: &TerrainConfig,
    terrain_map: &TerrainMapData,
    texture_slots: Option<&TerrainTextureSlotsData>,
    world_name: &str,
    terrain_grass_slots: Option<&HashSet<u8>>,
) -> Vec<PreparedTerrainBatch> {
    let width = heightmap.width as usize;
    let height = heightmap.height as usize;
    if width < 2 || height < 2 {
        return Vec::new();
    }

    let scale = config.size.scale;
    let vertical_scale = config.height_multiplier * (scale / config.legacy_terrain_scale.max(1.0));
    let layer_uv_scale = config
        .texture_layers
        .first()
        .map(|layer| layer.scale.max(0.01))
        .unwrap_or(1.0);
    let default_uv_step = TERRAIN_TILE_UV_STEP / layer_uv_scale;

    let mut positions = Vec::with_capacity(width * height);
    let mut min_height = f32::MAX;
    let mut max_height = f32::MIN;

    for z in 0..height {
        for x in 0..width {
            let h = heightmap.get_height(x, z) * vertical_scale;
            min_height = min_height.min(h);
            max_height = max_height.max(h);
            positions.push([x as f32 * scale, h, z as f32 * scale]);
        }
    }

    let grid_indices = build_grid_indices(width, height);
    let normals = calculate_normals(&positions, &grid_indices);
    let vertex_lights = compute_vertex_lights(width, height);

    let map_width = terrain_map.width().min(width);
    let map_height = terrain_map.height().min(height);
    if map_width < 2 || map_height < 2 {
        warn!(
            "Terrain map dimensions ({}, {}) are invalid for mesh dimensions ({}, {})",
            terrain_map.width(),
            terrain_map.height(),
            width,
            height
        );
        return Vec::new();
    }

    let mut buckets: HashMap<TerrainBatchKey, TerrainBatch> = HashMap::new();
    let mut slot_uv_steps: HashMap<u8, [f32; 2]> = HashMap::new();
    let mut grass_billboard_like_cache: HashMap<u8, bool> = HashMap::new();
    let fallback_ground_slot = find_fallback_ground_slot(world_name, texture_slots);
    let mut replaced_billboard_grass_bases = 0usize;
    let mut skipped_billboard_grass_alpha_layers = 0usize;

    for z in 0..(map_height - 1) {
        for x in 0..(map_width - 1) {
            let s1 = terrain_map.sample(x, z).unwrap_or(TerrainMapSample {
                layer1: 0,
                layer2: TERRAIN_NO_LAYER_SLOT,
                alpha: 0,
            });
            let s2 = terrain_map.sample(x + 1, z).unwrap_or(s1);
            let s3 = terrain_map.sample(x + 1, z + 1).unwrap_or(s1);
            let s4 = terrain_map.sample(x, z + 1).unwrap_or(s1);

            let alphas = [s1.alpha, s2.alpha, s3.alpha, s4.alpha];
            let is_opaque = alphas.iter().all(|alpha| *alpha == 255);
            let has_alpha = alphas.iter().any(|alpha| *alpha > 0);

            let i1 = z * width + x;
            let i2 = i1 + 1;
            let i4 = (z + 1) * width + x;
            let i3 = i4 + 1;

            let mut base_slot = if is_opaque {
                if s1.layer2 != TERRAIN_NO_LAYER_SLOT {
                    s1.layer2
                } else {
                    s1.layer1
                }
            } else {
                s1.layer1
            };

            if base_slot != TERRAIN_NO_LAYER_SLOT
                && is_billboard_like_grass_slot(
                    base_slot,
                    terrain_grass_slots,
                    world_name,
                    texture_slots,
                    &mut grass_billboard_like_cache,
                )
            {
                let original_base_slot = base_slot;
                let layer2_is_grass = terrain_grass_slots
                    .map(|grass_slots| grass_slots.contains(&s1.layer2))
                    .unwrap_or(false);
                if s1.layer2 != TERRAIN_NO_LAYER_SLOT && !layer2_is_grass {
                    base_slot = s1.layer2;
                } else if let Some(fallback_slot) = fallback_ground_slot {
                    base_slot = fallback_slot;
                }
                if base_slot != original_base_slot {
                    replaced_billboard_grass_bases += 1;
                }
            }

            if base_slot != TERRAIN_NO_LAYER_SLOT {
                let uv_step = *slot_uv_steps.entry(base_slot).or_insert_with(|| {
                    resolve_texture_uv_step(
                        world_name,
                        texture_slots,
                        base_slot,
                        default_uv_step,
                        layer_uv_scale,
                    )
                });
                let tile_uvs = tile_uvs(x, z, uv_step);
                let batch = buckets.entry(TerrainBatchKey {
                    texture_slot: base_slot,
                    pass: TerrainPass::Base,
                });
                push_tile(
                    batch.or_default(),
                    [i1, i2, i3, i4],
                    [255, 255, 255, 255],
                    tile_uvs,
                    &positions,
                    &normals,
                    &vertex_lights,
                );
            }

            if !is_opaque
                && has_alpha
                && s1.layer2 != TERRAIN_NO_LAYER_SLOT
                && s1.layer2 != base_slot
            {
                // Billboard-like grass belongs to the dedicated grass pass, not alpha terrain blends.
                if is_billboard_like_grass_slot(
                    s1.layer2,
                    terrain_grass_slots,
                    world_name,
                    texture_slots,
                    &mut grass_billboard_like_cache,
                ) {
                    skipped_billboard_grass_alpha_layers += 1;
                    continue;
                }

                let uv_step = *slot_uv_steps.entry(s1.layer2).or_insert_with(|| {
                    resolve_texture_uv_step(
                        world_name,
                        texture_slots,
                        s1.layer2,
                        default_uv_step,
                        layer_uv_scale,
                    )
                });
                let tile_uvs = tile_uvs(x, z, uv_step);
                let batch = buckets.entry(TerrainBatchKey {
                    texture_slot: s1.layer2,
                    pass: TerrainPass::Alpha,
                });
                push_tile(
                    batch.or_default(),
                    [i1, i2, i3, i4],
                    alphas,
                    tile_uvs,
                    &positions,
                    &normals,
                    &vertex_lights,
                );
            }
        }
    }

    let mut prepared = Vec::new();

    for (key, batch) in buckets {
        if batch.positions.is_empty() {
            continue;
        }

        let Some(texture_path) =
            resolve_texture_slot_path(world_name, texture_slots, key.texture_slot)
        else {
            warn!(
                "Terrain texture slot {} has no resolved texture in '{}'",
                key.texture_slot, world_name
            );
            continue;
        };

        let triangles = batch.triangles();
        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, default());
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, batch.positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, batch.normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, batch.uvs);
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, batch.colors);

        prepared.push(PreparedTerrainBatch {
            mesh,
            texture_path,
            texture_slot: key.texture_slot,
            pass: key.pass,
            triangles,
        });
    }

    prepared.sort_by_key(|batch| (batch.pass, batch.texture_slot));

    if replaced_billboard_grass_bases > 0 {
        warn!(
            "Terrain '{}' replaced {} billboard-like grass base tiles with fallback ground slots",
            world_name, replaced_billboard_grass_bases
        );
    }
    if skipped_billboard_grass_alpha_layers > 0 {
        warn!(
            "Terrain '{}' skipped {} billboard-like grass alpha overlays",
            world_name, skipped_billboard_grass_alpha_layers
        );
    }

    info!(
        "Terrain mesh build: {}x{} vertices, map {}x{}, height range [{:.2}, {:.2}], batches={}",
        width,
        height,
        map_width,
        map_height,
        min_height,
        max_height,
        prepared.len()
    );

    prepared
}

fn find_fallback_ground_slot(
    world_name: &str,
    texture_slots: Option<&TerrainTextureSlotsData>,
) -> Option<u8> {
    for slot in [2_u8, 3_u8, 4_u8] {
        if resolve_texture_slot_path(world_name, texture_slots, slot).is_some() {
            return Some(slot);
        }
    }
    None
}

fn is_billboard_like_grass_slot(
    slot: u8,
    terrain_grass_slots: Option<&HashSet<u8>>,
    world_name: &str,
    texture_slots: Option<&TerrainTextureSlotsData>,
    cache: &mut HashMap<u8, bool>,
) -> bool {
    let Some(grass_slots) = terrain_grass_slots else {
        return false;
    };
    if !grass_slots.contains(&slot) {
        return false;
    }

    *cache.entry(slot).or_insert_with(|| {
        let Some(texture_path) = resolve_texture_slot_path(world_name, texture_slots, slot) else {
            return false;
        };
        let full_path = Path::new(CLIENT_ASSETS_ROOT).join(texture_path);
        let Some((width, height)) = read_png_dimensions(&full_path) else {
            return false;
        };
        height > 0 && width >= height * 2
    })
}

fn push_tile(
    batch: &mut TerrainBatch,
    indices: [usize; 4],
    alphas: [u8; 4],
    tile_uvs: [[f32; 2]; 4],
    positions: &[[f32; 3]],
    normals: &[[f32; 3]],
    vertex_lights: &[[f32; 3]],
) {
    // Triangle order matches MU terrain face winding.
    let triangles = [
        (indices[0], alphas[0], tile_uvs[0]),
        (indices[3], alphas[3], tile_uvs[3]),
        (indices[1], alphas[1], tile_uvs[1]),
        (indices[1], alphas[1], tile_uvs[1]),
        (indices[3], alphas[3], tile_uvs[3]),
        (indices[2], alphas[2], tile_uvs[2]),
    ];

    for (vertex_index, alpha, uv) in triangles {
        let light = vertex_lights[vertex_index];
        batch.positions.push(positions[vertex_index]);
        batch.normals.push(normals[vertex_index]);
        batch
            .colors
            .push([light[0], light[1], light[2], alpha as f32 / 255.0]);
        batch.uvs.push(uv);
    }
}

fn tile_uvs(x: usize, z: usize, uv_step: [f32; 2]) -> [[f32; 2]; 4] {
    let su = x as f32 * uv_step[0];
    let sv = z as f32 * uv_step[1];
    [
        [su, sv],
        [su + uv_step[0], sv],
        [su + uv_step[0], sv + uv_step[1]],
        [su, sv + uv_step[1]],
    ]
}

fn resolve_texture_uv_step(
    world_name: &str,
    texture_slots: Option<&TerrainTextureSlotsData>,
    texture_slot: u8,
    default_uv_step: f32,
    layer_uv_scale: f32,
) -> [f32; 2] {
    let Some(texture_path) = resolve_texture_slot_path(world_name, texture_slots, texture_slot)
    else {
        return [default_uv_step, default_uv_step];
    };

    let full_path = Path::new(CLIENT_ASSETS_ROOT).join(texture_path);
    let Some((width, height)) = read_png_dimensions(&full_path) else {
        return [default_uv_step, default_uv_step];
    };

    if width == 0 || height == 0 {
        return [default_uv_step, default_uv_step];
    }

    [
        (64.0 / width as f32) / layer_uv_scale,
        (64.0 / height as f32) / layer_uv_scale,
    ]
}

fn read_png_dimensions(path: &Path) -> Option<(u32, u32)> {
    if !path.is_file() {
        return None;
    }
    let bytes = fs::read(path).ok()?;
    if bytes.len() < 24 || bytes[0..8] != *b"\x89PNG\r\n\x1a\n" || bytes[12..16] != *b"IHDR" {
        return None;
    }
    let width = u32::from_be_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);
    let height = u32::from_be_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]);
    Some((width, height))
}

fn compute_vertex_lights(width: usize, height: usize) -> Vec<[f32; 3]> {
    vec![[1.0, 1.0, 1.0]; width * height]
}

fn build_grid_indices(width: usize, height: usize) -> Vec<u32> {
    let mut indices = Vec::with_capacity((width - 1) * (height - 1) * 6);
    for z in 0..(height - 1) {
        for x in 0..(width - 1) {
            let top_left = (z * width + x) as u32;
            let top_right = top_left + 1;
            let bottom_left = ((z + 1) * width + x) as u32;
            let bottom_right = bottom_left + 1;

            indices.push(top_left);
            indices.push(bottom_left);
            indices.push(top_right);
            indices.push(top_right);
            indices.push(bottom_left);
            indices.push(bottom_right);
        }
    }
    indices
}

fn calculate_normals(positions: &[[f32; 3]], indices: &[u32]) -> Vec<[f32; 3]> {
    let mut normals = vec![[0.0f32, 0.0, 0.0]; positions.len()];

    for triangle in indices.chunks(3) {
        let i0 = triangle[0] as usize;
        let i1 = triangle[1] as usize;
        let i2 = triangle[2] as usize;

        let p0 = Vec3::from(positions[i0]);
        let p1 = Vec3::from(positions[i1]);
        let p2 = Vec3::from(positions[i2]);

        let edge1 = p1 - p0;
        let edge2 = p2 - p0;
        let normal = edge1.cross(edge2).normalize_or_zero();

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

    for normal in &mut normals {
        let n = Vec3::from(*normal).normalize_or_zero();
        let n = if n.length_squared() > 0.0 { n } else { Vec3::Y };
        *normal = [n.x, n.y, n.z];
    }

    normals
}

fn resolve_texture_slot_path(
    world_name: &str,
    texture_slots: Option<&TerrainTextureSlotsData>,
    texture_slot: u8,
) -> Option<String> {
    if let Some(texture_slots) = texture_slots {
        if let Some(path) = texture_slots.path_for_slot(texture_slot) {
            if let Some(resolved) = resolve_existing_asset_path(path) {
                return Some(resolved);
            }
        }
    }

    for candidate in default_texture_slot_candidates(world_name, texture_slot) {
        if let Some(resolved) = resolve_existing_asset_path(&candidate) {
            return Some(resolved);
        }
    }

    None
}

fn default_texture_slot_candidates(world_name: &str, texture_slot: u8) -> Vec<String> {
    let mut candidates = Vec::new();
    let push_texture = |values: &mut Vec<String>, name: &str| {
        values.push(format!("data/{world_name}/{name}.png"));
        values.push(format!("data/{world_name}/{}.png", to_snake_case(name)));
        values.push(format!("data/{world_name}/{}.png", name.to_lowercase()));
    };

    match texture_slot {
        0 => push_texture(&mut candidates, "TileGrass01"),
        1 => push_texture(&mut candidates, "TileGrass02"),
        2 => push_texture(&mut candidates, "TileGround01"),
        3 => push_texture(&mut candidates, "TileGround02"),
        4 => push_texture(&mut candidates, "TileGround03"),
        5 => push_texture(&mut candidates, "TileWater01"),
        6 => push_texture(&mut candidates, "TileWood01"),
        7 => push_texture(&mut candidates, "TileRock01"),
        8 => push_texture(&mut candidates, "TileRock02"),
        9 => push_texture(&mut candidates, "TileRock03"),
        10 => push_texture(&mut candidates, "TileRock04"),
        11 => push_texture(&mut candidates, "TileRock05"),
        12 => push_texture(&mut candidates, "TileRock06"),
        13 => push_texture(&mut candidates, "TileRock07"),
        14..=29 => {
            let ext_id = texture_slot - 13;
            push_texture(&mut candidates, &format!("ExtTile{ext_id:02}"));
            push_texture(&mut candidates, &format!("ext_tile{ext_id:02}"));
        }
        30 => push_texture(&mut candidates, "TileGrass01"),
        31 => push_texture(&mut candidates, "TileGrass02"),
        32 => push_texture(&mut candidates, "TileGrass03"),
        _ => {}
    }

    candidates
}

fn to_snake_case(value: &str) -> String {
    let mut result = String::new();
    for (index, ch) in value.chars().enumerate() {
        if ch.is_ascii_uppercase() {
            if index != 0 {
                result.push('_');
            }
            result.push(ch.to_ascii_lowercase());
        } else {
            result.push(ch);
        }
    }
    result
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene_runtime::scene_loader::{HeightmapData, TerrainConfig};
    use serde::de::DeserializeOwned;
    use std::fs;

    fn load_json_asset<T: DeserializeOwned>(relative_path: &str) -> T {
        let full_path = Path::new(CLIENT_ASSETS_ROOT).join(relative_path);
        let bytes = fs::read(&full_path)
            .unwrap_or_else(|err| panic!("failed to read asset {}: {err}", full_path.display()));
        serde_json::from_slice(&bytes).unwrap_or_else(|err| {
            panic!("failed to parse JSON asset {}: {err}", full_path.display())
        })
    }

    #[test]
    fn world4_skips_billboard_like_grass_alpha_overlays() {
        let world_name = "world4";
        let config: TerrainConfig = load_json_asset("data/world4/terrain_config.json");
        let heightmap: HeightmapData = load_json_asset("data/world4/terrain_height.json");
        let terrain_map: TerrainMapData = load_json_asset("data/world4/terrain_map.json");
        let texture_slots: TerrainTextureSlotsData =
            load_json_asset("data/world4/terrain_texture_slots.json");

        let grass_slots = find_grass_slots(Some(&texture_slots), world_name);
        assert!(
            !grass_slots.is_empty(),
            "world4 must expose grass slots to validate terrain grass handling"
        );

        let batches = build_terrain_batches(
            &heightmap,
            &config,
            &terrain_map,
            Some(&texture_slots),
            world_name,
            Some(&grass_slots),
        );

        let mut cache = HashMap::new();
        let billboard_like_alpha_batches = batches
            .iter()
            .filter(|batch| batch.pass == TerrainPass::Alpha)
            .filter(|batch| {
                is_billboard_like_grass_slot(
                    batch.texture_slot,
                    Some(&grass_slots),
                    world_name,
                    Some(&texture_slots),
                    &mut cache,
                )
            })
            .count();

        assert_eq!(
            billboard_like_alpha_batches, 0,
            "world4 should not render billboard-like grass in terrain alpha pass"
        );
    }
}
