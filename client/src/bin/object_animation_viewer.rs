use bevy::asset::{AssetId, AssetPlugin};
#[cfg(feature = "solari")]
use bevy::camera::CameraMainTextureUsages;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::gltf::{Gltf, GltfMaterialExtras};
use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::light::GlobalAmbientLight;
use bevy::mesh::skinning::SkinnedMesh;
use bevy::mesh::VertexAttributeValues;
use bevy::pbr::MaterialPlugin;
use bevy::prelude::*;
#[cfg(feature = "solari")]
use bevy::render::render_resource::TextureUsages;
#[cfg(feature = "solari")]
use bevy::solari::prelude::{RaytracingMesh3d, SolariLighting};
use bevy::window::WindowResolution;
use bevy_egui::input::EguiWantsInput;
use bevy_egui::{EguiClipboard, EguiContexts, EguiPlugin, EguiPrimaryContextPass, egui};
use serde_json::Value;
#[path = "../bevy_compat.rs"]
mod bevy_compat;
#[path = "../legacy_additive.rs"]
mod legacy_additive;
use bevy_compat::*;
use legacy_additive::{
    LegacyAdditiveMaterial, legacy_additive_from_standard, legacy_additive_intensity_from_extras,
};
use std::collections::{HashMap, HashSet};
use std::time::Duration;

const DEFAULT_OBJECT_PLAYBACK_SPEED: f32 = 0.16;
const CAMERA_MIN_DISTANCE: f32 = 80.0;
const CAMERA_MAX_DISTANCE: f32 = 4_000.0;
const CAMERA_ZOOM_SPEED: f32 = 120.0;
const GROUND_Y_OFFSET: f32 = 0.5;

#[derive(Resource)]
struct ViewerState {
    input_path: String,
    loaded_scene_path: Option<String>,
    loaded_gltf_path: Option<String>,
    scene_entity: Option<Entity>,
    gltf_handle: Option<Handle<Gltf>>,
    graph_handle: Option<Handle<AnimationGraph>>,
    animation_nodes: Vec<AnimationNodeIndex>,
    animation_names: Vec<String>,
    selected_animation: usize,
    playback_speed: f32,
    playing: bool,
    pending_load: bool,
    pending_apply_selection: bool,
    pending_toggle_playback: bool,
    animations_initialized: bool,
    status: String,
    #[cfg(feature = "solari")]
    use_raytracing: bool,
    #[cfg(feature = "solari")]
    pending_rt_change: bool,
}

impl Default for ViewerState {
    fn default() -> Self {
        Self {
            input_path: "data/object_4/object_40.glb".to_string(),
            loaded_scene_path: None,
            loaded_gltf_path: None,
            scene_entity: None,
            gltf_handle: None,
            graph_handle: None,
            animation_nodes: Vec::new(),
            animation_names: Vec::new(),
            selected_animation: 0,
            playback_speed: DEFAULT_OBJECT_PLAYBACK_SPEED,
            playing: true,
            pending_load: false,
            pending_apply_selection: false,
            pending_toggle_playback: false,
            animations_initialized: false,
            status: "Ready. Enter a .glb path and click Load.".to_string(),
            #[cfg(feature = "solari")]
            use_raytracing: true,
            #[cfg(feature = "solari")]
            pending_rt_change: false,
        }
    }
}

#[derive(Component)]
struct LoadedSceneRoot;

#[derive(Component)]
struct ViewerAnimationBound;

#[derive(Component)]
struct GroundPlane;

#[derive(Component)]
struct OrbitCamera {
    target: Vec3,
    yaw: f32,
    pitch: f32,
    distance: f32,
    min_distance: f32,
    max_distance: f32,
    yaw_speed: f32,
    pitch_speed: f32,
    zoom_speed: f32,
}

#[derive(Resource, Default)]
struct MeshBoundsCache {
    local_aabb_by_mesh: HashMap<AssetId<Mesh>, (Vec3, Vec3)>,
}

fn main() {
    let mut app = App::new();
    app.insert_resource(GlobalAmbientLight {
        color: Color::WHITE,
        brightness: 250.0,
        affects_lightmapped_meshes: true,
    })
    .insert_resource(MeshBoundsCache::default())
    .insert_resource(ViewerState::default())
    .add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "MU Object Animation Viewer".to_string(),
                    resolution: WindowResolution::new(1440, 900),
                    resizable: true,
                    ..default()
                }),
                ..default()
            })
            .set(AssetPlugin {
                file_path: asset_root_path().into(),
                ..default()
            }),
    )
    .add_plugins(MaterialPlugin::<LegacyAdditiveMaterial>::default())
    .add_plugins(EguiPlugin::default());

    #[cfg(feature = "solari")]
    app.add_plugins(bevy::solari::SolariPlugins);

    app.add_systems(Startup, setup_viewer_scene)
        .add_systems(EguiPrimaryContextPass, draw_ui_panel)
        .add_systems(
            Update,
            (
                handle_load_request,
                initialize_animation_graph,
                apply_legacy_gltf_material_overrides_for_viewer,
                bind_animation_players,
                apply_animation_controls,
                sync_ground_below_loaded_object,
                update_orbit_camera,
            ),
        );

    #[cfg(feature = "solari")]
    app.add_systems(Update, toggle_raytracing);

    app.run();
}

fn asset_root_path() -> String {
    format!("{}/../assets", env!("CARGO_MANIFEST_DIR"))
}

fn setup_viewer_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut camera_transform = Transform::default();
    let orbit_camera = OrbitCamera {
        target: Vec3::new(0.0, 120.0, 0.0),
        yaw: std::f32::consts::FRAC_PI_2,
        pitch: 0.28,
        distance: 854.0,
        min_distance: CAMERA_MIN_DISTANCE,
        max_distance: CAMERA_MAX_DISTANCE,
        yaw_speed: 1.8,
        pitch_speed: 1.4,
        zoom_speed: CAMERA_ZOOM_SPEED,
    };
    apply_orbit_transform(&mut camera_transform, &orbit_camera);

    let mut camera = commands.spawn((
        Camera3dBundle {
            transform: camera_transform,
            tonemapping: Tonemapping::ReinhardLuminance,
            ..default()
        },
        orbit_camera,
    ));

    #[cfg(feature = "solari")]
    camera.insert((
        SolariLighting::default(),
        Msaa::Off,
        CameraMainTextureUsages::default().with(TextureUsages::STORAGE_BINDING),
    ));

    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            illuminance: 5000.0,
            #[cfg(feature = "solari")]
            shadows_enabled: false,
            #[cfg(not(feature = "solari"))]
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -1.0, 0.8, 0.0)),
        ..default()
    });

    commands
        .spawn(PbrBundle {
            mesh: Mesh3d(meshes.add(Plane3d::default().mesh().size(5000.0, 5000.0))),
            material: MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(0.08, 0.09, 0.1),
                perceptual_roughness: 0.95,
                metallic: 0.0,
                ..default()
            })),
            ..default()
        })
        .insert(GroundPlane);
}

fn draw_ui_panel(
    mut contexts: EguiContexts,
    mut viewer: ResMut<ViewerState>,
    keys: Res<ButtonInput<KeyCode>>,
    mut clipboard: ResMut<EguiClipboard>,
) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };
    egui::Window::new("Object Loader")
        .default_pos(egui::pos2(12.0, 12.0))
        .default_width(520.0)
        .show(ctx, |ui| {
            ui.label("GLB path (relative to assets root)");
            let previous_path = viewer.input_path.clone();
            let path_edit = egui::TextEdit::singleline(&mut viewer.input_path)
                .id_source("object_loader_glb_path")
                .desired_width(f32::INFINITY)
                .show(ui);

            let command_pressed =
                command_modifier_pressed(&keys) || ctx.input(|i| i.modifiers.command);
            if path_edit.response.has_focus() && command_pressed {
                if keys.just_pressed(KeyCode::KeyC) {
                    clipboard.set_text(&viewer.input_path);
                    // Prevent stray "c" insertion when some platforms don't emit Copy/Paste events.
                    viewer.input_path = previous_path.clone();
                } else if keys.just_pressed(KeyCode::KeyX) {
                    clipboard.set_text(&viewer.input_path);
                    viewer.input_path.clear();
                } else if keys.just_pressed(KeyCode::KeyV) {
                    if let Some(contents) = clipboard.get_text() {
                        viewer.input_path = contents;
                    } else {
                        viewer.input_path = previous_path.clone();
                        viewer.status =
                            "Clipboard unavailable for paste on this platform/session.".to_string();
                    }
                }
            }

            ui.horizontal(|ui| {
                if ui.button("Paste").clicked() {
                    if let Some(contents) = clipboard.get_text() {
                        viewer.input_path = contents;
                    } else {
                        viewer.status =
                            "Clipboard unavailable for paste on this platform/session.".to_string();
                    }
                }
                if ui.button("Copy").clicked() {
                    clipboard.set_text(&viewer.input_path);
                }
            });

            ui.horizontal(|ui| {
                if ui.button("Load").clicked() {
                    viewer.pending_load = true;
                }

                let can_play = !viewer.animation_nodes.is_empty();
                let play_label = if viewer.playing { "Pause" } else { "Play" };
                if ui
                    .add_enabled(can_play, egui::Button::new(play_label))
                    .clicked()
                {
                    viewer.pending_toggle_playback = true;
                }
            });

            let speed_slider =
                egui::Slider::new(&mut viewer.playback_speed, 0.02..=1.2).text("Playback speed");
            if ui.add(speed_slider).changed() {
                viewer.pending_apply_selection = true;
            }
            ui.label("W/S: pitch | A/D: yaw | Scroll: zoom");

            if viewer.animation_names.is_empty() {
                ui.label("Animations: none loaded");
            } else {
                let mut selected_animation = viewer.selected_animation;
                egui::ComboBox::from_label("Animation")
                    .selected_text(
                        viewer
                            .animation_names
                            .get(viewer.selected_animation)
                            .cloned()
                            .unwrap_or_else(|| format!("Animation {}", viewer.selected_animation)),
                    )
                    .show_ui(ui, |ui| {
                        for (index, name) in viewer.animation_names.iter().enumerate() {
                            ui.selectable_value(&mut selected_animation, index, name);
                        }
                    });

                if selected_animation != viewer.selected_animation {
                    viewer.selected_animation = selected_animation;
                    viewer.pending_apply_selection = true;
                }
            }

            #[cfg(feature = "solari")]
            {
                let prev_rt = viewer.use_raytracing;
                ui.checkbox(&mut viewer.use_raytracing, "Raytracing (Solari)");
                if viewer.use_raytracing != prev_rt {
                    viewer.pending_rt_change = true;
                }
            }

            if let Some(path) = &viewer.loaded_scene_path {
                ui.label(format!("Scene: {path}"));
            }
            if let Some(path) = &viewer.loaded_gltf_path {
                ui.label(format!("Asset: {path}"));
            }

            ui.separator();
            ui.label(format!("Status: {}", viewer.status));
            ui.label(format!("Assets root: {}", asset_root_path()));
        });
}

fn handle_load_request(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    existing_roots: Query<Entity, With<LoadedSceneRoot>>,
    mut viewer: ResMut<ViewerState>,
) {
    if !viewer.pending_load {
        return;
    }
    viewer.pending_load = false;

    let mut roots_to_despawn = HashSet::new();
    if let Some(entity) = viewer.scene_entity.take() {
        roots_to_despawn.insert(entity);
    }
    roots_to_despawn.extend(existing_roots.iter());
    for entity in roots_to_despawn {
        commands.entity(entity).try_despawn();
    }

    viewer.graph_handle = None;
    viewer.gltf_handle = None;
    viewer.animation_nodes.clear();
    viewer.animation_names.clear();
    viewer.selected_animation = 0;
    viewer.pending_apply_selection = false;
    viewer.pending_toggle_playback = false;
    viewer.animations_initialized = false;
    viewer.playing = true;

    let raw_path = viewer.input_path.trim();
    if raw_path.is_empty() {
        viewer.status = "Path is empty.".to_string();
        return;
    }

    let (scene_path, gltf_path) = normalize_scene_and_gltf_path(raw_path);
    let scene_handle: Handle<Scene> = asset_server.load(scene_path.clone());
    let gltf_handle: Handle<Gltf> = asset_server.load(gltf_path.clone());

    let scene_entity = commands
        .spawn((
            SceneBundle {
                scene: SceneRoot(scene_handle),
                ..default()
            },
            LoadedSceneRoot,
        ))
        .id();

    viewer.scene_entity = Some(scene_entity);
    viewer.loaded_scene_path = Some(scene_path);
    viewer.loaded_gltf_path = Some(gltf_path);
    viewer.gltf_handle = Some(gltf_handle);
    viewer.status = "Loading scene and animations...".to_string();
}

fn initialize_animation_graph(
    mut viewer: ResMut<ViewerState>,
    gltfs: Res<Assets<Gltf>>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
) {
    if viewer.animations_initialized {
        return;
    }

    let Some(gltf_handle) = viewer.gltf_handle.clone() else {
        return;
    };
    let Some(gltf) = gltfs.get(&gltf_handle) else {
        return;
    };

    viewer.animations_initialized = true;

    if gltf.animations.is_empty() {
        viewer.status = "Loaded scene has no animations.".to_string();
        return;
    }

    let mut graph = AnimationGraph::new();
    let animation_nodes: Vec<AnimationNodeIndex> = graph
        .add_clips(gltf.animations.iter().cloned(), 1.0, graph.root)
        .collect();

    let mut names = Vec::with_capacity(gltf.animations.len());
    names.extend((0..gltf.animations.len()).map(|index| format!("Animation {index}")));

    let index_by_clip_id: HashMap<AssetId<AnimationClip>, usize> = gltf
        .animations
        .iter()
        .enumerate()
        .map(|(index, handle)| (handle.id(), index))
        .collect();

    for (name, handle) in &gltf.named_animations {
        if let Some(index) = index_by_clip_id.get(&handle.id()) {
            names[*index] = name.to_string();
        }
    }

    viewer.graph_handle = Some(graphs.add(graph));
    viewer.animation_nodes = animation_nodes;
    viewer.animation_names = names;
    viewer.selected_animation = 0;
    viewer.pending_apply_selection = true;
    viewer.status = format!("Loaded {} animation(s).", viewer.animation_nodes.len());
}

fn bind_animation_players(
    mut commands: Commands,
    mut players: Query<(Entity, &mut AnimationPlayer), Without<ViewerAnimationBound>>,
    viewer: Res<ViewerState>,
) {
    let Some(graph_handle) = viewer.graph_handle.clone() else {
        return;
    };
    let Some(animation_node) = viewer
        .animation_nodes
        .get(viewer.selected_animation)
        .copied()
    else {
        return;
    };

    for (entity, mut player) in &mut players {
        let mut transitions = AnimationTransitions::new();
        transitions
            .play(&mut player, animation_node, Duration::ZERO)
            .set_speed(viewer.playback_speed.max(0.001))
            .repeat();
        if !viewer.playing {
            player.pause_all();
        }

        commands.entity(entity).insert((
            AnimationGraphHandle(graph_handle.clone()),
            transitions,
            ViewerAnimationBound,
        ));
    }
}

fn apply_animation_controls(
    mut viewer: ResMut<ViewerState>,
    mut players: Query<
        (&mut AnimationPlayer, &mut AnimationTransitions),
        With<ViewerAnimationBound>,
    >,
) {
    let selection_changed = std::mem::take(&mut viewer.pending_apply_selection);
    let toggle_requested = std::mem::take(&mut viewer.pending_toggle_playback);

    if toggle_requested {
        viewer.playing = !viewer.playing;
    }

    if !selection_changed && !toggle_requested {
        return;
    }

    let Some(animation_node) = viewer
        .animation_nodes
        .get(viewer.selected_animation)
        .copied()
    else {
        viewer.status = "No animation available to control.".to_string();
        return;
    };

    for (mut player, mut transitions) in &mut players {
        if selection_changed {
            transitions
                .play(&mut player, animation_node, Duration::ZERO)
                .set_speed(viewer.playback_speed.max(0.001))
                .repeat();
        }

        if toggle_requested || selection_changed {
            if viewer.playing {
                player.resume_all();
            } else {
                player.pause_all();
            }
        }
    }

    if selection_changed {
        let selected_name = viewer
            .animation_names
            .get(viewer.selected_animation)
            .map(String::as_str)
            .unwrap_or("unnamed");
        viewer.status = format!(
            "Playing animation {} ({selected_name}).",
            viewer.selected_animation
        );
    } else if toggle_requested {
        viewer.status = if viewer.playing {
            "Playback resumed.".to_string()
        } else {
            "Playback paused.".to_string()
        };
    }
}

fn sync_ground_below_loaded_object(
    viewer: Res<ViewerState>,
    mut grounds: Query<&mut Transform, With<GroundPlane>>,
    mesh_entities: Query<(Entity, &GlobalTransform, &Mesh3d), Without<GroundPlane>>,
    children_query: Query<&Children>,
    meshes: Res<Assets<Mesh>>,
    mut bounds_cache: ResMut<MeshBoundsCache>,
) {
    let Some(scene_root) = viewer.scene_entity else {
        return;
    };
    let Ok(mut ground_transform) = grounds.single_mut() else {
        return;
    };

    let mut scene_entities = HashSet::new();
    collect_descendants(scene_root, &children_query, &mut scene_entities);
    scene_entities.insert(scene_root);

    let mut min_world_y = f32::INFINITY;
    for (entity, global_transform, mesh_handle) in &mesh_entities {
        if !scene_entities.contains(&entity) {
            continue;
        }

        let bounds = if let Some(bounds) = bounds_cache.local_aabb_by_mesh.get(&mesh_handle.0.id())
        {
            Some(*bounds)
        } else {
            let Some(mesh) = meshes.get(&mesh_handle.0) else {
                continue;
            };
            let Some(bounds) = compute_local_aabb(mesh) else {
                continue;
            };
            bounds_cache
                .local_aabb_by_mesh
                .insert(mesh_handle.0.id(), bounds);
            Some(bounds)
        };

        let Some((local_min, local_max)) = bounds else {
            continue;
        };
        let entity_min_y = transformed_aabb_min_y(global_transform, local_min, local_max);
        min_world_y = min_world_y.min(entity_min_y);
    }

    if min_world_y.is_finite() {
        ground_transform.translation.y = min_world_y - GROUND_Y_OFFSET;
    }
}

fn collect_descendants(root: Entity, children_query: &Query<&Children>, out: &mut HashSet<Entity>) {
    let Ok(children) = children_query.get(root) else {
        return;
    };
    for child in children.iter() {
        if out.insert(child) {
            collect_descendants(child, children_query, out);
        }
    }
}

fn compute_local_aabb(mesh: &Mesh) -> Option<(Vec3, Vec3)> {
    let positions = mesh.attribute(Mesh::ATTRIBUTE_POSITION)?;
    let VertexAttributeValues::Float32x3(positions) = positions else {
        return None;
    };

    let mut min = Vec3::splat(f32::INFINITY);
    let mut max = Vec3::splat(f32::NEG_INFINITY);
    for [x, y, z] in positions {
        let p = Vec3::new(*x, *y, *z);
        min = min.min(p);
        max = max.max(p);
    }

    if min.x.is_finite() && min.y.is_finite() && min.z.is_finite() {
        Some((min, max))
    } else {
        None
    }
}

fn transformed_aabb_min_y(transform: &GlobalTransform, local_min: Vec3, local_max: Vec3) -> f32 {
    let affine = transform.affine();
    let xs = [local_min.x, local_max.x];
    let ys = [local_min.y, local_max.y];
    let zs = [local_min.z, local_max.z];

    let mut min_world_y = f32::INFINITY;
    for x in xs {
        for y in ys {
            for z in zs {
                let world = affine.transform_point3(Vec3::new(x, y, z));
                min_world_y = min_world_y.min(world.y);
            }
        }
    }
    min_world_y
}

#[cfg(feature = "solari")]
fn toggle_raytracing(
    mut commands: Commands,
    mut viewer: ResMut<ViewerState>,
    new_meshes: Query<(Entity, &Mesh3d), Added<Mesh3d>>,
    all_meshes: Query<(Entity, &Mesh3d)>,
    rt_query: Query<Entity, With<RaytracingMesh3d>>,
) {
    let toggled = std::mem::take(&mut viewer.pending_rt_change);

    if viewer.use_raytracing {
        // Auto-tag newly spawned meshes with RaytracingMesh3d
        for (entity, mesh3d) in &new_meshes {
            commands
                .entity(entity)
                .insert(RaytracingMesh3d(mesh3d.0.clone()));
        }
        // When just toggled on, tag ALL existing meshes
        if toggled {
            for (entity, mesh3d) in &all_meshes {
                commands
                    .entity(entity)
                    .insert(RaytracingMesh3d(mesh3d.0.clone()));
            }
            viewer.status = "Raytracing enabled (Solari)".to_string();
        }
    } else if toggled {
        for entity in &rt_query {
            commands.entity(entity).remove::<RaytracingMesh3d>();
        }
        viewer.status = "Raytracing disabled".to_string();
    }
}

fn update_orbit_camera(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    egui_wants_input: Res<EguiWantsInput>,
    mut mouse_wheel_events: MessageReader<MouseWheel>,
    mut cameras: Query<(&mut Transform, &mut OrbitCamera)>,
) {
    let wants_keyboard_input = egui_wants_input.wants_any_keyboard_input();
    let wants_pointer_input = egui_wants_input.wants_any_pointer_input();

    let mut yaw_input = 0.0f32;
    if !wants_keyboard_input {
        if keys.pressed(KeyCode::KeyA) {
            yaw_input -= 1.0;
        }
        if keys.pressed(KeyCode::KeyD) {
            yaw_input += 1.0;
        }
    }

    let mut pitch_input = 0.0f32;
    if !wants_keyboard_input {
        if keys.pressed(KeyCode::KeyW) {
            pitch_input += 1.0;
        }
        if keys.pressed(KeyCode::KeyS) {
            pitch_input -= 1.0;
        }
    }

    let mut zoom_input = 0.0f32;
    for event in mouse_wheel_events.read() {
        if wants_pointer_input {
            continue;
        }
        let unit_scale = match event.unit {
            MouseScrollUnit::Line => 1.0,
            MouseScrollUnit::Pixel => 0.02,
        };
        zoom_input += event.y * unit_scale;
    }

    if yaw_input == 0.0 && pitch_input == 0.0 && zoom_input == 0.0 {
        return;
    }

    let dt = time.delta_secs();
    for (mut transform, mut orbit) in &mut cameras {
        orbit.distance = (orbit.distance - zoom_input * orbit.zoom_speed)
            .clamp(orbit.min_distance, orbit.max_distance);
        orbit.yaw += yaw_input * orbit.yaw_speed * dt;
        orbit.pitch = (orbit.pitch + pitch_input * orbit.pitch_speed * dt).clamp(-1.2, 1.2);
        apply_orbit_transform(&mut transform, &orbit);
    }
}

fn apply_orbit_transform(transform: &mut Transform, orbit: &OrbitCamera) {
    let cos_pitch = orbit.pitch.cos();
    let direction = Vec3::new(
        orbit.yaw.cos() * cos_pitch,
        orbit.pitch.sin(),
        orbit.yaw.sin() * cos_pitch,
    );
    transform.translation = orbit.target + direction * orbit.distance;
    transform.look_at(orbit.target, Vec3::Y);
}

fn command_modifier_pressed(keys: &ButtonInput<KeyCode>) -> bool {
    keys.pressed(KeyCode::ControlLeft)
        || keys.pressed(KeyCode::ControlRight)
        || keys.pressed(KeyCode::SuperLeft)
        || keys.pressed(KeyCode::SuperRight)
}

fn normalize_scene_and_gltf_path(raw_path: &str) -> (String, String) {
    let normalized = raw_path.trim().replace('\\', "/");
    let scene_path = if normalized.contains('#') {
        normalized.clone()
    } else if normalized.to_ascii_lowercase().ends_with(".glb")
        || normalized.to_ascii_lowercase().ends_with(".gltf")
    {
        format!("{normalized}#Scene0")
    } else {
        normalized.clone()
    };

    let gltf_path = scene_path
        .split('#')
        .next()
        .unwrap_or(&scene_path)
        .to_string();
    (scene_path, gltf_path)
}

fn apply_legacy_gltf_material_overrides_for_viewer(
    mut commands: Commands,
    mut legacy_materials: ResMut<Assets<LegacyAdditiveMaterial>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    query: Query<
        (
            Entity,
            &MeshMaterial3d<StandardMaterial>,
            &GltfMaterialExtras,
            Option<&SkinnedMesh>,
        ),
        Added<GltfMaterialExtras>,
    >,
) {
    for (entity, material_handle, extras, skinned_mesh) in &query {
        let Ok(payload) = serde_json::from_str::<Value>(&extras.value) else {
            continue;
        };

        let Some(blend_mode) = payload.get("mu_legacy_blend_mode").and_then(Value::as_str) else {
            continue;
        };
        if blend_mode != "additive" {
            continue;
        }

        let Some(material) = materials.get(&material_handle.0).cloned() else {
            continue;
        };

        let intensity = legacy_additive_intensity_from_extras(&payload);
        if skinned_mesh.is_some() {
            let mut additive_standard = material.clone();
            additive_standard.alpha_mode = AlphaMode::Add;
            additive_standard.double_sided = true;
            additive_standard.cull_mode = None;
            additive_standard.unlit = true;
            additive_standard.emissive = LinearRgba::rgb(intensity, intensity, intensity);
            additive_standard.emissive_texture = additive_standard
                .emissive_texture
                .clone()
                .or_else(|| additive_standard.base_color_texture.clone());
            let has_texture = additive_standard.base_color_texture.is_some()
                || additive_standard.emissive_texture.is_some();
            let additive_handle = materials.add(additive_standard);
            commands
                .entity(entity)
                .insert(MeshMaterial3d(additive_handle));

            debug!(
                "object_viewer: applied legacy additive override (object={:?}/{:?}) texture={} intensity={:.2} material=StandardMaterial(Add, skinned)",
                payload
                    .get("mu_legacy_object_dir")
                    .and_then(|value| value.as_i64()),
                payload
                    .get("mu_legacy_object_model")
                    .and_then(|value| value.as_i64()),
                has_texture,
                intensity,
            );
        } else {
            let mut legacy_material = legacy_additive_from_standard(&material);
            legacy_material.params.intensity = intensity;
            let has_texture = legacy_material.color_texture.is_some();
            let legacy_material_handle = legacy_materials.add(legacy_material);

            commands
                .entity(entity)
                .remove::<MeshMaterial3d<StandardMaterial>>()
                .insert(MeshMaterial3d(legacy_material_handle));

            debug!(
                "object_viewer: applied legacy additive override (object={:?}/{:?}) texture={} intensity={:.2} material=LegacyAdditiveMaterial",
                payload
                    .get("mu_legacy_object_dir")
                    .and_then(|value| value.as_i64()),
                payload
                    .get("mu_legacy_object_model")
                    .and_then(|value| value.as_i64()),
                has_texture,
                intensity,
            );
        }
    }
}
