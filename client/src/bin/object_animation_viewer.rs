use bevy::asset::{AssetId, AssetPlugin};
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::gltf::Gltf;
use bevy::prelude::*;
use bevy::window::WindowResolution;
use bevy_egui::{EguiContexts, EguiPlugin, egui};
use std::collections::HashMap;
use std::time::Duration;

const DEFAULT_OBJECT_PLAYBACK_SPEED: f32 = 0.16;

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
}

impl Default for ViewerState {
    fn default() -> Self {
        Self {
            input_path: "data/object4/object40.glb".to_string(),
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
        }
    }
}

#[derive(Component)]
struct LoadedSceneRoot;

#[derive(Component)]
struct ViewerAnimationBound;

#[derive(Component)]
struct OrbitCamera {
    target: Vec3,
    yaw: f32,
    pitch: f32,
    distance: f32,
    yaw_speed: f32,
    pitch_speed: f32,
}

fn main() {
    App::new()
        .insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 250.0,
        })
        .insert_resource(ViewerState::default())
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "MU Object Animation Viewer".to_string(),
                        resolution: WindowResolution::new(1440.0, 900.0),
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
        .add_plugins(EguiPlugin)
        .add_systems(Startup, setup_viewer_scene)
        .add_systems(
            Update,
            (
                draw_ui_panel,
                handle_load_request,
                initialize_animation_graph,
                bind_animation_players,
                apply_animation_controls,
                update_orbit_camera,
            ),
        )
        .run();
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
        yaw_speed: 1.8,
        pitch_speed: 1.4,
    };
    apply_orbit_transform(&mut camera_transform, &orbit_camera);

    commands.spawn((
        Camera3dBundle {
            transform: camera_transform,
            tonemapping: Tonemapping::ReinhardLuminance,
            ..default()
        },
        orbit_camera,
    ));

    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            illuminance: 18_000.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -1.0, 0.8, 0.0)),
        ..default()
    });

    commands.spawn(PbrBundle {
        mesh: meshes.add(Plane3d::default().mesh().size(5000.0, 5000.0)),
        material: materials.add(StandardMaterial {
            base_color: Color::srgb(0.08, 0.09, 0.1),
            perceptual_roughness: 0.95,
            metallic: 0.0,
            ..default()
        }),
        ..default()
    });
}

fn draw_ui_panel(mut contexts: EguiContexts, mut viewer: ResMut<ViewerState>) {
    let ctx = contexts.ctx_mut();
    egui::Window::new("Object Loader")
        .default_pos(egui::pos2(12.0, 12.0))
        .default_width(520.0)
        .show(ctx, |ui| {
            ui.label("GLB path (relative to assets root)");
            ui.text_edit_singleline(&mut viewer.input_path);

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
            ui.label("W/S: pitch | A/D: yaw");

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

    if let Some(entity) = viewer.scene_entity.take() {
        commands.entity(entity).despawn_recursive();
    }
    for entity in &existing_roots {
        commands.entity(entity).despawn_recursive();
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
                scene: scene_handle,
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

        commands
            .entity(entity)
            .insert((graph_handle.clone(), transitions, ViewerAnimationBound));
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

fn update_orbit_camera(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut egui_contexts: EguiContexts,
    mut cameras: Query<(&mut Transform, &mut OrbitCamera)>,
) {
    if egui_contexts.ctx_mut().wants_keyboard_input() {
        return;
    }

    let mut yaw_input = 0.0f32;
    if keys.pressed(KeyCode::KeyA) {
        yaw_input -= 1.0;
    }
    if keys.pressed(KeyCode::KeyD) {
        yaw_input += 1.0;
    }

    let mut pitch_input = 0.0f32;
    if keys.pressed(KeyCode::KeyW) {
        pitch_input += 1.0;
    }
    if keys.pressed(KeyCode::KeyS) {
        pitch_input -= 1.0;
    }

    if yaw_input == 0.0 && pitch_input == 0.0 {
        return;
    }

    let dt = time.delta_seconds();
    for (mut transform, mut orbit) in &mut cameras {
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
