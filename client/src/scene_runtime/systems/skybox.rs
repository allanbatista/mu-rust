use crate::bevy_compat::*;
use crate::infra::assets::{asset_path_exists, resolve_asset_path};
use crate::scene_runtime::components::*;
use crate::scene_runtime::state::RuntimeSceneAssets;
use bevy::image::{ImageAddressMode, ImageLoaderSettings, ImageSampler, ImageSamplerDescriptor};
use bevy::light::{NotShadowCaster, NotShadowReceiver};
use bevy::math::primitives::Cuboid;
use bevy::prelude::*;
use bevy::render::render_resource::Face;
use bevy::render::render_resource::FilterMode;
use std::fs;
use std::path::{Path, PathBuf};

const CLIENT_ASSETS_ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../assets");
const SKYBOX_SIZE: f32 = 60_000.0;

#[derive(Component)]
pub struct SkyboxSpawned;

/// Spawn an unlit skybox mesh parented to the main 3D camera.
pub fn spawn_skybox_when_ready(
    mut commands: Commands,
    assets: Res<RuntimeSceneAssets>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    skybox_query: Query<&SkyboxSpawned>,
    camera_query: Query<Entity, With<Camera3d>>,
) {
    if !assets.loaded || !skybox_query.is_empty() {
        return;
    }

    let Ok(camera_entity) = camera_query.single() else {
        return;
    };

    let world_name = assets
        .world
        .as_ref()
        .map(|world| world.world_name.as_str())
        .unwrap_or(assets.world_name.as_str());
    let sky_texture_path = resolve_skybox_texture_path(world_name);

    let material_handle = if let Some(texture_path) = sky_texture_path.clone() {
        let resolved_texture_path = resolve_asset_path(&texture_path);
        let texture = asset_server.load_with_settings(resolved_texture_path, |settings: &mut _| {
            *settings = ImageLoaderSettings {
                is_srgb: true,
                sampler: ImageSampler::Descriptor(ImageSamplerDescriptor {
                    address_mode_u: ImageAddressMode::ClampToEdge,
                    address_mode_v: ImageAddressMode::ClampToEdge,
                    mag_filter: FilterMode::Linear.into(),
                    min_filter: FilterMode::Linear.into(),
                    mipmap_filter: FilterMode::Linear.into(),
                    ..default()
                }),
                ..default()
            };
        });

        materials.add(StandardMaterial {
            base_color_texture: Some(texture),
            unlit: true,
            cull_mode: Some(Face::Front),
            perceptual_roughness: 1.0,
            metallic: 0.0,
            reflectance: 0.0,
            ..default()
        })
    } else {
        materials.add(StandardMaterial {
            base_color: Color::srgb(0.50, 0.66, 0.86),
            unlit: true,
            cull_mode: Some(Face::Front),
            ..default()
        })
    };

    let mesh_handle = meshes.add(Mesh::from(Cuboid::new(
        SKYBOX_SIZE,
        SKYBOX_SIZE,
        SKYBOX_SIZE,
    )));

    commands.entity(camera_entity).with_children(|parent| {
        parent.spawn((
            RuntimeSceneEntity,
            SkyboxSpawned,
            NotShadowCaster,
            NotShadowReceiver,
            PbrBundle {
                mesh: Mesh3d(mesh_handle),
                material: MeshMaterial3d(material_handle),
                transform: Transform::IDENTITY,
                ..default()
            },
        ));
    });

    if let Some(texture_path) = sky_texture_path {
        info!("Skybox spawned for '{}' using {}", world_name, texture_path);
    } else {
        warn!(
            "Skybox spawned for '{}' with solid fallback color (no texture found)",
            world_name
        );
    }
}

fn resolve_skybox_texture_path(world_name: &str) -> Option<String> {
    for candidate in skybox_texture_candidates(world_name) {
        if let Some(resolved) = resolve_existing_asset_path(&candidate) {
            return Some(resolved);
        }
    }
    None
}

fn skybox_texture_candidates(world_name: &str) -> Vec<String> {
    vec![
        // CC0 panorama (Poly Haven via Wikimedia) bundled as high-quality default.
        "sky/hausdorf_clear_sky_2560.jpg".to_string(),
        format!("data/{world_name}/map1.png"),
        format!("data/{world_name}/map2.png"),
        format!("data/{world_name}/map_form.png"),
        format!("data/{world_name}/terrain_light.png"),
        format!("data/{world_name}/terrainlight.png"),
        "data/logo/sos3sky01.png".to_string(),
        "data/logo/sos3sky02.png".to_string(),
        "data/logo/cloud.png".to_string(),
        "data/logo/bkcloud.png".to_string(),
        "data/effect/clouds.png".to_string(),
        "data/effect/clouds4.png".to_string(),
    ]
}

fn resolve_existing_asset_path(raw_path: &str) -> Option<String> {
    let normalized = normalize_asset_path(raw_path);
    if normalized.is_empty() {
        return None;
    }

    if asset_path_exists(&normalized) {
        return Some(resolve_asset_path(&normalized));
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
