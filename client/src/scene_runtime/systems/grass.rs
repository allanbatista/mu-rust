use super::terrain::TerrainSpawned;
use crate::scene_runtime::components::*;
use crate::scene_runtime::state::RuntimeSceneAssets;
use bevy::pbr::{
    MaterialMeshBundle, MaterialPipeline, MaterialPipelineKey, NotShadowCaster, NotShadowReceiver,
};
use bevy::prelude::*;
use bevy::render::mesh::{MeshVertexBufferLayoutRef, PrimitiveTopology};
use bevy::render::render_resource::{
    AsBindGroup, RenderPipelineDescriptor, ShaderRef, ShaderType, SpecializedMeshPipelineError,
};
use bevy::render::texture::{
    ImageAddressMode, ImageLoaderSettings, ImageSampler, ImageSamplerDescriptor,
};
use std::collections::HashSet;
use std::path::Path;

const GRASS_HEIGHT: f32 = 100.0;
const GRASS_WIDTH: f32 = 80.0;
const GRASS_ALPHA_CUTOFF: f32 = 0.35;
const GRASS_WIND_STRENGTH: f32 = 15.0;
const GRASS_WIND_SPEED: f32 = 1.5;
const CLIENT_ASSETS_ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../assets");

/// Marker component to track if terrain grass has been spawned.
#[derive(Component)]
pub struct TerrainGrassSpawned;

/// Custom material for grass billboards with wind animation and unlit rendering.
#[derive(Asset, AsBindGroup, TypePath, Debug, Clone)]
pub struct GrassMaterial {
    #[texture(0)]
    #[sampler(1)]
    pub base_color_texture: Option<Handle<Image>>,
    #[uniform(2)]
    pub params: GrassShaderParams,
}

#[derive(ShaderType, Debug, Clone, Copy)]
pub struct GrassShaderParams {
    pub alpha_cutoff: f32,
    pub wind_strength: f32,
    pub wind_speed: f32,
    pub _padding: f32,
}

impl Material for GrassMaterial {
    fn vertex_shader() -> ShaderRef {
        "shaders/grass_billboard.wgsl".into()
    }

    fn fragment_shader() -> ShaderRef {
        "shaders/grass_billboard.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Mask(self.params.alpha_cutoff)
    }

    fn specialize(
        _pipeline: &MaterialPipeline<Self>,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        _key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        descriptor.primitive.cull_mode = None;
        Ok(())
    }
}

/// System to spawn grass billboards once terrain is ready.
pub fn spawn_terrain_grass_when_ready(
    mut commands: Commands,
    assets: Res<RuntimeSceneAssets>,
    terrain_configs: Res<Assets<TerrainConfig>>,
    heightmaps: Res<Assets<HeightmapData>>,
    terrain_maps: Res<Assets<TerrainMapData>>,
    terrain_texture_slots: Res<Assets<TerrainTextureSlotsData>>,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut grass_materials: ResMut<Assets<GrassMaterial>>,
    terrain_query: Query<&TerrainSpawned>,
    grass_query: Query<&TerrainGrassSpawned>,
) {
    // Guard: grass already spawned or terrain not ready yet
    if !grass_query.is_empty() || terrain_query.is_empty() || !assets.loaded {
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

    // Identify grass texture slots
    let grass_slots = find_grass_slots(texture_slots_data, &world.world_name);
    if grass_slots.is_empty() {
        info!(
            "No grass texture slots found for '{}', skipping grass",
            world.world_name
        );
        commands.spawn((RuntimeSceneEntity, TerrainGrassSpawned));
        return;
    }

    // Find the primary grass texture path
    let grass_texture_path =
        find_grass_texture_path(texture_slots_data, &world.world_name, &grass_slots);
    let Some(grass_texture_path) = grass_texture_path else {
        warn!(
            "Could not resolve grass texture for '{}', skipping grass",
            world.world_name
        );
        commands.spawn((RuntimeSceneEntity, TerrainGrassSpawned));
        return;
    };

    // Build the batched grass mesh
    let scale = config.size.scale;
    let vertical_scale = config.height_multiplier * (scale / config.legacy_terrain_scale.max(1.0));

    let map_width = terrain_map.width().min(heightmap.width as usize);
    let map_height = terrain_map.height().min(heightmap.height as usize);

    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut normals: Vec<[f32; 3]> = Vec::new();
    let mut uvs: Vec<[f32; 2]> = Vec::new();
    let mut grass_count = 0u32;

    for z in 0..map_height.saturating_sub(1) {
        for x in 0..map_width.saturating_sub(1) {
            let s1 = terrain_map.sample(x, z).unwrap_or(TerrainMapSample {
                layer1: 0,
                layer2: 255,
                alpha: 0,
            });
            let s2 = terrain_map.sample(x + 1, z).unwrap_or(s1);
            let s3 = terrain_map.sample(x + 1, z + 1).unwrap_or(s1);
            let s4 = terrain_map.sample(x, z + 1).unwrap_or(s1);

            if !grass_slots.contains(&s1.layer1) {
                continue;
            }

            if s1.alpha > 0 || s2.alpha > 0 || s3.alpha > 0 || s4.alpha > 0 {
                continue;
            }

            let cx = (x as f32 + 0.5) * scale;
            let cz = (z as f32 + 0.5) * scale;
            let cy = heightmap.get_height(x, z) * vertical_scale;

            // Select texture variant (4 variants in a 256x64 atlas, each 64x64)
            let variant = ((x * 7 + z * 13) % 4) as f32;
            let u_min = variant * 0.25;
            let u_max = (variant + 1.0) * 0.25;

            let half_w = GRASS_WIDTH * 0.5;
            let h = GRASS_HEIGHT;

            // Quad 1: diagonal along +X+Z to -X-Z
            let diag = half_w * std::f32::consts::FRAC_1_SQRT_2;
            push_quad(
                &mut positions,
                &mut normals,
                &mut uvs,
                [cx - diag, cy, cz - diag],
                [cx + diag, cy, cz + diag],
                [cx + diag, cy + h, cz + diag],
                [cx - diag, cy + h, cz - diag],
                u_min,
                u_max,
            );

            // Quad 2: perpendicular diagonal along +X-Z to -X+Z
            push_quad(
                &mut positions,
                &mut normals,
                &mut uvs,
                [cx + diag, cy, cz - diag],
                [cx - diag, cy, cz + diag],
                [cx - diag, cy + h, cz + diag],
                [cx + diag, cy + h, cz - diag],
                u_min,
                u_max,
            );

            grass_count += 1;
        }
    }

    if positions.is_empty() {
        info!(
            "No grass cells found in '{}', skipping grass mesh",
            world.world_name
        );
        commands.spawn((RuntimeSceneEntity, TerrainGrassSpawned));
        return;
    }

    let triangles = positions.len() / 3;
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);

    let mesh_handle = meshes.add(mesh);

    let diffuse =
        asset_server.load_with_settings(grass_texture_path.clone(), |settings: &mut _| {
            *settings = ImageLoaderSettings {
                is_srgb: true,
                sampler: ImageSampler::Descriptor(ImageSamplerDescriptor {
                    address_mode_u: ImageAddressMode::ClampToEdge,
                    address_mode_v: ImageAddressMode::ClampToEdge,
                    ..default()
                }),
                ..default()
            };
        });

    let material_handle = grass_materials.add(GrassMaterial {
        base_color_texture: Some(diffuse),
        params: GrassShaderParams {
            alpha_cutoff: GRASS_ALPHA_CUTOFF,
            wind_strength: GRASS_WIND_STRENGTH,
            wind_speed: GRASS_WIND_SPEED,
            _padding: 0.0,
        },
    });

    let root = commands
        .spawn((
            RuntimeSceneEntity,
            TerrainGrassSpawned,
            Terrain,
            SpatialBundle::default(),
        ))
        .id();

    commands.entity(root).with_children(|parent| {
        parent.spawn((
            RuntimeSceneEntity,
            Terrain,
            NotShadowCaster,
            NotShadowReceiver,
            MaterialMeshBundle::<GrassMaterial> {
                mesh: mesh_handle,
                material: material_handle,
                transform: Transform::IDENTITY,
                ..default()
            },
        ));
    });

    info!(
        "Terrain grass '{}' spawned: {} cells, {} triangles, texture={} (wind enabled)",
        world.world_name, grass_count, triangles, grass_texture_path
    );
}

/// Push a single quad (2 triangles, 6 vertices) into the buffers.
fn push_quad(
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    uvs: &mut Vec<[f32; 2]>,
    bl: [f32; 3], // bottom-left
    br: [f32; 3], // bottom-right
    tr: [f32; 3], // top-right
    tl: [f32; 3], // top-left
    u_min: f32,
    u_max: f32,
) {
    let up = [0.0, 1.0, 0.0];

    // Triangle 1: bl, br, tr
    positions.push(bl);
    positions.push(br);
    positions.push(tr);
    normals.push(up);
    normals.push(up);
    normals.push(up);
    uvs.push([u_min, 1.0]);
    uvs.push([u_max, 1.0]);
    uvs.push([u_max, 0.0]);

    // Triangle 2: bl, tr, tl
    positions.push(bl);
    positions.push(tr);
    positions.push(tl);
    normals.push(up);
    normals.push(up);
    normals.push(up);
    uvs.push([u_min, 1.0]);
    uvs.push([u_max, 0.0]);
    uvs.push([u_min, 0.0]);
}

/// Identify which texture slots correspond to grass textures.
pub(super) fn find_grass_slots(
    texture_slots: Option<&TerrainTextureSlotsData>,
    world_name: &str,
) -> HashSet<u8> {
    let mut grass_slots = HashSet::new();

    if let Some(slots) = texture_slots {
        for (&slot_id, path) in &slots.slots {
            if path.to_lowercase().contains("grass") {
                grass_slots.insert(slot_id as u8);
            }
        }
    }

    // Fallback: default MU terrain convention — slots 0, 1 are TileGrass01/02,
    // slots 30, 31, 32 are TileGrass01/02/03
    if grass_slots.is_empty() {
        for candidate_slot in [0u8, 1, 30, 31, 32] {
            let candidates = default_grass_candidates(world_name, candidate_slot);
            for candidate in candidates {
                let full = Path::new(CLIENT_ASSETS_ROOT).join(&candidate);
                if full.is_file() {
                    grass_slots.insert(candidate_slot);
                    break;
                }
            }
        }
    }

    grass_slots
}

/// Find the path to the primary grass billboard texture.
fn find_grass_texture_path(
    texture_slots: Option<&TerrainTextureSlotsData>,
    world_name: &str,
    grass_slots: &HashSet<u8>,
) -> Option<String> {
    // Try the first grass slot from texture_slots data
    if let Some(slots) = texture_slots {
        let mut slot_ids: Vec<u8> = grass_slots.iter().copied().collect();
        slot_ids.sort();
        for slot_id in &slot_ids {
            if let Some(path) = slots.path_for_slot(*slot_id) {
                let normalized = path.trim().replace('\\', "/");
                let normalized = normalized.trim_start_matches('/');
                let full = Path::new(CLIENT_ASSETS_ROOT).join(normalized);
                if full.is_file() {
                    return Some(normalized.to_string());
                }
                // Try case-insensitive
                if let Some(resolved) = resolve_case_insensitive(normalized) {
                    return Some(resolved);
                }
            }
        }
    }

    // Fallback: look for tile_grass files directly
    for name in ["tile_grass01", "tile_grass03", "tile_grass02"] {
        let path = format!("data/{world_name}/{name}.png");
        let full = Path::new(CLIENT_ASSETS_ROOT).join(&path);
        if full.is_file() {
            return Some(path);
        }
        // Try other casing
        for cased in [
            format!("data/{world_name}/TileGrass01.png"),
            format!("data/{world_name}/TileGrass03.png"),
            format!("data/{world_name}/TileGrass02.png"),
        ] {
            let full = Path::new(CLIENT_ASSETS_ROOT).join(&cased);
            if full.is_file() {
                return Some(cased);
            }
        }
    }

    None
}

fn default_grass_candidates(world_name: &str, slot: u8) -> Vec<String> {
    let names: Vec<&str> = match slot {
        0 | 30 => vec!["TileGrass01", "tile_grass01"],
        1 | 31 => vec!["TileGrass02", "tile_grass02"],
        32 => vec!["TileGrass03", "tile_grass03"],
        _ => vec![],
    };
    names
        .into_iter()
        .map(|n| format!("data/{world_name}/{n}.png"))
        .collect()
}

fn resolve_case_insensitive(rel_path: &str) -> Option<String> {
    let root = Path::new(CLIENT_ASSETS_ROOT);
    let full = root.join(rel_path);
    if full.is_file() {
        return Some(rel_path.to_string());
    }

    // Walk path components case-insensitively
    let mut current = root.to_path_buf();
    let mut resolved_parts: Vec<String> = Vec::new();

    for part in rel_path.split('/').filter(|s| !s.is_empty()) {
        let direct = current.join(part);
        if direct.exists() {
            current = direct;
            resolved_parts.push(part.to_string());
            continue;
        }

        let needle = part.to_lowercase();
        let mut matched = false;
        if let Ok(entries) = std::fs::read_dir(&current) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.to_lowercase() == needle {
                    current = entry.path();
                    resolved_parts.push(name);
                    matched = true;
                    break;
                }
            }
        }
        if !matched {
            return None;
        }
    }

    if current.is_file() {
        Some(resolved_parts.join("/"))
    } else {
        None
    }
}
