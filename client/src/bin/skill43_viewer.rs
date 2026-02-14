use bevy::asset::AssetPlugin;
use bevy::camera::ClearColorConfig;
use bevy::color::LinearRgba;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::input::mouse::MouseWheel;
use bevy::light::{NotShadowCaster, NotShadowReceiver};
use bevy::mesh::PrimitiveTopology;
use bevy::prelude::*;
use bevy::sprite::Anchor;
use bevy::window::{PrimaryWindow, WindowResolution};
use bevy_egui::input::EguiWantsInput;
use bevy_egui::{egui, EguiContexts, EguiPlugin, EguiPrimaryContextPass};
use rand::Rng;
use std::collections::VecDeque;
use std::f32::consts::PI;
use std::path::Path;

const REQUIRED_ASSET_REAR_TEX: &str = "data/effect/n_skill.png";
const REQUIRED_ASSET_FORWARD_TEX: &str = "data/effect/joint_sword_red.png";
const REQUIRED_ASSET_THUNDER_TEX: &str = "data/effect/joint_thunder_01.png";
const REQUIRED_ASSET_COMBO_TEX: &str = "data/effect/flashing.png";
const REQUIRED_ASSET_BACKGROUND_TEX: &str = "prototype/noria-placeholder.png";
const MU_TICK_RATE: f32 = 25.0;
const MU_TICK_SECONDS: f32 = 1.0 / MU_TICK_RATE;

#[derive(Resource, Default)]
struct MouseTarget {
    world: Vec3,
    valid: bool,
}

#[derive(Resource, Default)]
struct Skill43State {
    active_cast: Option<ActiveCast>,
    impact_ticks_remaining: i32,
    impact_center: Vec3,
    frame_ticks: u32,
    move_scene_frame: i32,
    next_seed: i32,
    current_sword_tip: Vec3,
}

#[derive(Clone, Copy)]
struct ActiveCast {
    attack_time: i32,
    forward: Vec3,
    target_pos: Vec3,
    impact_started: bool,
}

#[derive(Resource)]
struct FxAssets {
    quad_mesh: Handle<Mesh>,
    rear_texture: Handle<Image>,
    forward_texture: Handle<Image>,
    thunder_texture: Handle<Image>,
    combo_texture: Handle<Image>,
}

#[derive(Resource)]
struct CameraRig {
    distance: f32,
}

#[derive(Resource, Default)]
struct FpsOverlay {
    accumulator: f32,
    frames: u32,
    value: f32,
}

#[derive(Resource, Default)]
struct Skill43Clock {
    accumulator_seconds: f32,
    pending_ticks: u32,
}

#[derive(Component)]
struct MainCamera;

#[derive(Component)]
struct SceneBackdrop;

#[derive(Component)]
struct SceneBackdropImage {
    handle: Handle<Image>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum JointKind {
    RearSubType2,
    ForwardSpear,
    ThunderArc,
}

#[derive(Clone, Copy)]
struct TailSample {
    corners: [Vec3; 4],
}

#[derive(Component)]
struct Skill43Joint {
    kind: JointKind,
    lifetime_ticks: f32,
    max_lifetime_ticks: f32,
    tail_max: usize,
    position: Vec3,
    start_position: Vec3,
    target_position: Vec3,
    direction: Vec3,
    light: Vec3,
    base_width: f32,
    seed: i32,
    uv_scroll_speed: f32,
    tails: VecDeque<TailSample>,
}

#[derive(Component)]
struct JointRender {
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
    emissive_strength: f32,
}

#[derive(Component)]
struct ComboSprite {
    material: Handle<StandardMaterial>,
    lifetime: f32,
    max_lifetime: f32,
    scale_start: f32,
    scale_end: f32,
    emissive_strength: f32,
    color: Vec3,
}

#[derive(Resource, Clone)]
struct Skill43Tuning {
    animation_speed: f32,
    cast_plane_y: f32,
    max_ticks_per_update: u32,
    limit_attack_time: i32,
    rear_phase_start: i32,
    rear_phase_end: i32,
    rear_spawns_per_tick: u32,
    rear_spawn_radius: f32,
    rear_spawn_height: f32,
    rear_spawn_backward: f32,
    sword_tip_height: f32,
    sword_tip_forward: f32,
    rear_lifetime_ticks: f32,
    rear_tail_max: usize,
    rear_magic_scale: f32,
    rear_magic_speed_rate: f32,
    rear_width_scale: f32,
    rear_emissive: f32,
    forward_phase_start: i32,
    forward_phase_end: i32,
    forward_distance_base: f32,
    forward_distance_step: f32,
    forward_distance_time_pivot: i32,
    forward_lifetime_ticks: f32,
    forward_tail_max: usize,
    forward_speed_per_tick: f32,
    forward_width: f32,
    forward_emissive: f32,
    impact_start_attack_time: i32,
    impact_total_ticks: i32,
    impact_arcs_per_tick: u32,
    impact_center_height: f32,
    impact_offset_x: f32,
    impact_offset_y_min: f32,
    impact_offset_y_max: f32,
    impact_offset_z: f32,
    thunder_lifetime_ticks_min: f32,
    thunder_lifetime_ticks_max: f32,
    thunder_tail_max: usize,
    thunder_width_min: f32,
    thunder_width_max: f32,
    thunder_emissive: f32,
    thunder_uv_scroll_speed: f32,
    combo_lifetime_seconds: f32,
    combo_scale_start: f32,
    combo_scale_end: f32,
    combo_emissive: f32,
    camera_pitch_deg: f32,
    camera_yaw_deg: f32,
    camera_look_height: f32,
    camera_distance_default: f32,
    zoom_min: f32,
    zoom_max: f32,
    zoom_speed: f32,
}

impl Default for Skill43Tuning {
    fn default() -> Self {
        Self {
            animation_speed: 1.0,
            cast_plane_y: 0.0,
            max_ticks_per_update: 8,
            limit_attack_time: 15,
            rear_phase_start: 2,
            rear_phase_end: 8,
            rear_spawns_per_tick: 3,
            rear_spawn_radius: 300.0,
            rear_spawn_height: 120.0,
            rear_spawn_backward: 1400.0,
            sword_tip_height: 90.0,
            sword_tip_forward: 300.0,
            rear_lifetime_ticks: 20.0,
            rear_tail_max: 5,
            rear_magic_scale: 300.0,
            rear_magic_speed_rate: 1.4,
            rear_width_scale: 1.0,
            rear_emissive: 10.0,
            forward_phase_start: 6,
            forward_phase_end: 12,
            forward_distance_base: 100.0,
            forward_distance_step: 10.0,
            forward_distance_time_pivot: 8,
            forward_lifetime_ticks: 10.0,
            forward_tail_max: 6,
            forward_speed_per_tick: 14.0,
            forward_width: 22.0,
            forward_emissive: 12.0,
            impact_start_attack_time: 10,
            impact_total_ticks: 35,
            impact_arcs_per_tick: 4,
            impact_center_height: 80.0,
            impact_offset_x: 70.0,
            impact_offset_y_min: -35.0,
            impact_offset_y_max: 95.0,
            impact_offset_z: 40.0,
            thunder_lifetime_ticks_min: 8.0,
            thunder_lifetime_ticks_max: 16.0,
            thunder_tail_max: 10,
            thunder_width_min: 6.0,
            thunder_width_max: 16.0,
            thunder_emissive: 12.0,
            thunder_uv_scroll_speed: 2.2,
            combo_lifetime_seconds: 0.28,
            combo_scale_start: 90.0,
            combo_scale_end: 220.0,
            combo_emissive: 8.0,
            camera_pitch_deg: 48.5,
            camera_yaw_deg: -45.0,
            camera_look_height: 80.0,
            camera_distance_default: 1000.0,
            zoom_min: 300.0,
            zoom_max: 2400.0,
            zoom_speed: 90.0,
        }
    }
}

impl Skill43Tuning {
    fn sanitize(&mut self) {
        self.animation_speed = self.animation_speed.max(0.0);
        self.max_ticks_per_update = self.max_ticks_per_update.max(1);
        self.limit_attack_time = self.limit_attack_time.max(1);
        self.rear_spawns_per_tick = self.rear_spawns_per_tick.min(64);
        self.rear_lifetime_ticks = self.rear_lifetime_ticks.max(1.0);
        self.rear_tail_max = self.rear_tail_max.max(2);
        self.rear_magic_scale = self.rear_magic_scale.max(0.0);
        self.rear_magic_speed_rate = self.rear_magic_speed_rate.max(0.01);
        self.rear_width_scale = self.rear_width_scale.max(0.01);
        self.rear_emissive = self.rear_emissive.max(0.0);
        self.forward_lifetime_ticks = self.forward_lifetime_ticks.max(1.0);
        self.forward_tail_max = self.forward_tail_max.max(2);
        self.forward_width = self.forward_width.max(0.1);
        self.forward_emissive = self.forward_emissive.max(0.0);
        self.impact_total_ticks = self.impact_total_ticks.max(1);
        self.impact_arcs_per_tick = self.impact_arcs_per_tick.max(1).min(64);
        self.thunder_lifetime_ticks_min = self.thunder_lifetime_ticks_min.max(1.0);
        self.thunder_lifetime_ticks_max = self
            .thunder_lifetime_ticks_max
            .max(self.thunder_lifetime_ticks_min + 1.0);
        self.thunder_tail_max = self.thunder_tail_max.max(2);
        self.thunder_width_min = self.thunder_width_min.max(0.1);
        self.thunder_width_max = self.thunder_width_max.max(self.thunder_width_min + 0.1);
        self.thunder_emissive = self.thunder_emissive.max(0.0);
        self.thunder_uv_scroll_speed = self.thunder_uv_scroll_speed.max(0.0);
        self.combo_lifetime_seconds = self.combo_lifetime_seconds.max(0.01);
        self.combo_scale_start = self.combo_scale_start.max(0.01);
        self.combo_scale_end = self.combo_scale_end.max(0.01);
        self.combo_emissive = self.combo_emissive.max(0.0);
        self.zoom_min = self.zoom_min.max(10.0);
        if self.zoom_max < self.zoom_min + 1.0 {
            self.zoom_max = self.zoom_min + 1.0;
        }
        self.zoom_speed = self.zoom_speed.max(0.0);
    }
}

fn main() {
    let asset_root = format!("{}/../assets", env!("CARGO_MANIFEST_DIR"));
    validate_required_assets(&asset_root);

    let tuning = Skill43Tuning::default();
    let initial_distance = tuning.camera_distance_default;

    App::new()
        .insert_resource(ClearColor(Color::BLACK))
        .insert_resource(MouseTarget::default())
        .insert_resource(Skill43State::default())
        .insert_resource(Skill43Clock::default())
        .insert_resource(FpsOverlay::default())
        .insert_resource(tuning)
        .insert_resource(CameraRig {
            distance: initial_distance,
        })
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "skill43_viewer - Skill 43 (faithful ribbons)".to_string(),
                        resolution: WindowResolution::new(1280, 720),
                        resizable: true,
                        ..default()
                    }),
                    ..default()
                })
                .set(AssetPlugin {
                    file_path: asset_root.into(),
                    ..default()
                }),
        )
        .add_plugins(EguiPlugin::default())
        .add_systems(Startup, setup_scene)
        .add_systems(
            EguiPrimaryContextPass,
            (draw_settings_window, draw_fps_overlay),
        )
        .add_systems(
            Update,
            (
                update_mouse_target,
                update_scene_frame,
                update_fps_overlay,
                handle_right_click_cast,
                advance_skill43_timeline,
                advance_impact_thunder_timeline,
                update_skill43_joints,
                update_joint_meshes_and_materials,
                update_combo_sprites,
                update_camera_zoom,
                apply_camera_rig,
                fit_scene_backdrop,
            )
                .chain(),
        )
        .run();
}

fn validate_required_assets(asset_root: &str) {
    let required = [
        REQUIRED_ASSET_REAR_TEX,
        REQUIRED_ASSET_FORWARD_TEX,
        REQUIRED_ASSET_THUNDER_TEX,
        REQUIRED_ASSET_COMBO_TEX,
        REQUIRED_ASSET_BACKGROUND_TEX,
    ];

    let missing: Vec<&str> = required
        .iter()
        .copied()
        .filter(|relative| !Path::new(asset_root).join(relative).is_file())
        .collect();

    if !missing.is_empty() {
        panic!(
            "skill43_viewer requires exact Skill 43 assets. Missing:\n{}",
            missing.join("\n")
        );
    }
}

fn setup_scene(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let quad_mesh = meshes.add(Mesh::from(Rectangle::new(1.0, 1.0)));
    let background_texture = asset_server.load(REQUIRED_ASSET_BACKGROUND_TEX);

    commands.insert_resource(FxAssets {
        quad_mesh: quad_mesh.clone(),
        rear_texture: asset_server.load(REQUIRED_ASSET_REAR_TEX),
        forward_texture: asset_server.load(REQUIRED_ASSET_FORWARD_TEX),
        thunder_texture: asset_server.load(REQUIRED_ASSET_THUNDER_TEX),
        combo_texture: asset_server.load(REQUIRED_ASSET_COMBO_TEX),
    });

    commands.spawn((
        Camera2d,
        Camera {
            order: 0,
            ..default()
        },
    ));

    commands.spawn((
        SceneBackdrop,
        SceneBackdropImage {
            handle: background_texture.clone(),
        },
        Sprite {
            image: background_texture,
            custom_size: None,
            ..default()
        },
        Anchor::CENTER,
        Transform::default(),
    ));

    commands.spawn((
        MainCamera,
        Camera {
            order: 1,
            clear_color: ClearColorConfig::None,
            ..default()
        },
        Camera3d::default(),
        Tonemapping::ReinhardLuminance,
    ));

    commands.spawn((
        DirectionalLight {
            illuminance: 10_000.0,
            shadows_enabled: false,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -1.0, 0.8, 0.0)),
    ));
}

fn update_mouse_target(
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    tuning: Res<Skill43Tuning>,
    mut mouse_target: ResMut<MouseTarget>,
) {
    let Ok(window) = windows.single() else {
        mouse_target.valid = false;
        return;
    };
    let Some(cursor) = window.cursor_position() else {
        mouse_target.valid = false;
        return;
    };

    let Ok((camera, camera_transform)) = cameras.single() else {
        mouse_target.valid = false;
        return;
    };

    let Ok(ray) = camera.viewport_to_world(camera_transform, cursor) else {
        mouse_target.valid = false;
        return;
    };

    if ray.direction.y.abs() <= f32::EPSILON {
        mouse_target.valid = false;
        return;
    }

    let distance = (tuning.cast_plane_y - ray.origin.y) / ray.direction.y;
    if distance < 0.0 {
        mouse_target.valid = false;
        return;
    }

    mouse_target.world = ray.origin + ray.direction * distance;
    mouse_target.valid = true;
}

fn update_scene_frame(
    time: Res<Time>,
    tuning: Res<Skill43Tuning>,
    mut clock: ResMut<Skill43Clock>,
    mut state: ResMut<Skill43State>,
) {
    let dt = simulation_delta_seconds(&time, &tuning);
    if dt > 0.0 {
        clock.accumulator_seconds += dt;
    }

    while clock.accumulator_seconds >= MU_TICK_SECONDS {
        clock.accumulator_seconds -= MU_TICK_SECONDS;
        clock.pending_ticks = clock.pending_ticks.saturating_add(1);
    }

    let ticks_to_process = clock.pending_ticks.min(tuning.max_ticks_per_update);
    clock.pending_ticks -= ticks_to_process;
    state.frame_ticks = ticks_to_process;
    state.move_scene_frame = state.move_scene_frame.wrapping_add(ticks_to_process as i32);
}

fn handle_right_click_cast(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    egui_wants_input: Res<EguiWantsInput>,
    mouse_target: Res<MouseTarget>,
    tuning: Res<Skill43Tuning>,
    mut state: ResMut<Skill43State>,
) {
    if !mouse_buttons.just_pressed(MouseButton::Right) {
        return;
    }
    if egui_wants_input.wants_any_pointer_input() {
        return;
    }
    if state.active_cast.is_some() || !mouse_target.valid {
        return;
    }

    let caster = Vec3::new(0.0, tuning.cast_plane_y, 0.0);
    let mut forward = mouse_target.world - caster;
    forward.y = 0.0;
    if forward.length_squared() <= f32::EPSILON {
        forward = Vec3::new(0.0, 0.0, -1.0);
    } else {
        forward = forward.normalize();
    }

    state.active_cast = Some(ActiveCast {
        attack_time: 1,
        forward,
        target_pos: Vec3::new(
            mouse_target.world.x,
            tuning.cast_plane_y,
            mouse_target.world.z,
        ),
        impact_started: false,
    });
    state.impact_ticks_remaining = 0;
    state.current_sword_tip = sword_tip_anchor(caster, forward, &tuning);
}

fn advance_skill43_timeline(
    mut commands: Commands,
    tuning: Res<Skill43Tuning>,
    mut state: ResMut<Skill43State>,
    assets: Res<FxAssets>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let Some(mut cast) = state.active_cast else {
        return;
    };
    let ticks = state.frame_ticks;
    if ticks == 0 {
        return;
    }

    for _ in 0..ticks {
        let attack_time = cast.attack_time;
        if attack_time <= 8 {
            let caster = Vec3::new(0.0, tuning.cast_plane_y, 0.0);
            state.current_sword_tip = sword_tip_anchor(caster, cast.forward, &tuning);
        }

        if (tuning.rear_phase_start..=tuning.rear_phase_end).contains(&attack_time) {
            for _ in 0..tuning.rear_spawns_per_tick {
                let seed = next_seed(&mut state);
                spawn_rear_subtype2_joint(
                    &mut commands,
                    &assets,
                    &mut meshes,
                    &mut materials,
                    &tuning,
                    cast.forward,
                    seed,
                );
            }
        }

        if (tuning.forward_phase_start..=tuning.forward_phase_end).contains(&attack_time) {
            let seed = next_seed(&mut state);
            spawn_forward_spear_joint(
                &mut commands,
                &assets,
                &mut meshes,
                &mut materials,
                &tuning,
                cast.forward,
                attack_time,
                seed,
            );
        }

        if attack_time >= tuning.impact_start_attack_time && !cast.impact_started {
            cast.impact_started = true;
            state.impact_center =
                cast.target_pos + Vec3::new(0.0, tuning.impact_center_height, 0.0);
            state.impact_ticks_remaining = tuning.impact_total_ticks;
            spawn_combo_burst(
                &mut commands,
                &assets,
                &mut materials,
                &tuning,
                state.impact_center,
            );
        }

        if attack_time >= 12 {
            cast.attack_time = tuning.limit_attack_time;
        } else {
            cast.attack_time += 1;
        }

        if cast.attack_time >= tuning.limit_attack_time {
            state.active_cast = None;
            return;
        }
    }

    state.active_cast = Some(cast);
}

fn advance_impact_thunder_timeline(
    mut commands: Commands,
    tuning: Res<Skill43Tuning>,
    mut state: ResMut<Skill43State>,
    assets: Res<FxAssets>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if state.impact_ticks_remaining <= 0 {
        return;
    }
    let ticks = state.frame_ticks;
    if ticks == 0 {
        return;
    }

    for _ in 0..ticks {
        if state.impact_ticks_remaining <= 0 {
            break;
        }

        for _ in 0..tuning.impact_arcs_per_tick {
            let seed = next_seed(&mut state);
            spawn_thunder_arc_joint(
                &mut commands,
                &assets,
                &mut meshes,
                &mut materials,
                &tuning,
                state.impact_center,
                seed,
            );
        }

        state.impact_ticks_remaining -= 1;
    }
}

fn spawn_rear_subtype2_joint(
    commands: &mut Commands,
    assets: &FxAssets,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    tuning: &Skill43Tuning,
    forward: Vec3,
    seed: i32,
) {
    let caster = Vec3::new(0.0, tuning.cast_plane_y, 0.0);

    let mut rng = rand::thread_rng();
    let angle = rng.gen_range(0.0..(PI * 2.0));
    let radius = tuning.rear_spawn_radius * rng.gen_range(0.0..1.0f32).sqrt();

    let mut spawn = caster
        + Vec3::new(
            radius * angle.cos(),
            tuning.rear_spawn_height,
            radius * angle.sin(),
        );
    spawn -= forward * tuning.rear_spawn_backward;

    spawn_joint_entity(
        commands,
        meshes,
        materials,
        SpawnJointDesc {
            kind: JointKind::RearSubType2,
            texture: &assets.rear_texture,
            position: spawn,
            start_position: spawn,
            target_position: spawn,
            direction: forward,
            lifetime_ticks: tuning.rear_lifetime_ticks,
            tail_max: tuning.rear_tail_max,
            light: Vec3::new(1.0, 0.3, 0.3),
            base_width: 60.0,
            emissive: tuning.rear_emissive,
            uv_scroll_speed: 0.0,
            seed,
            alpha_mode: AlphaMode::Blend,
        },
    );
}

fn spawn_forward_spear_joint(
    commands: &mut Commands,
    assets: &FxAssets,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    tuning: &Skill43Tuning,
    forward: Vec3,
    attack_time: i32,
    seed: i32,
) {
    let caster = Vec3::new(0.0, tuning.cast_plane_y, 0.0);
    let sword_tip = sword_tip_anchor(caster, forward, tuning);
    let distance = tuning.forward_distance_base
        + ((attack_time - tuning.forward_distance_time_pivot) as f32)
            * tuning.forward_distance_step;
    let position = sword_tip + forward * distance;

    spawn_joint_entity(
        commands,
        meshes,
        materials,
        SpawnJointDesc {
            kind: JointKind::ForwardSpear,
            texture: &assets.forward_texture,
            position,
            start_position: position,
            target_position: position + forward * 250.0,
            direction: forward,
            lifetime_ticks: tuning.forward_lifetime_ticks,
            tail_max: tuning.forward_tail_max,
            light: Vec3::new(1.0, 1.0, 1.0),
            base_width: tuning.forward_width,
            emissive: tuning.forward_emissive,
            uv_scroll_speed: 0.0,
            seed,
            alpha_mode: AlphaMode::Blend,
        },
    );
}

fn spawn_thunder_arc_joint(
    commands: &mut Commands,
    assets: &FxAssets,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    tuning: &Skill43Tuning,
    center: Vec3,
    seed: i32,
) {
    let mut rng = rand::thread_rng();
    let start = center + random_impact_offset(&mut rng, tuning);
    let end = center + random_impact_offset(&mut rng, tuning);
    let direction = (end - start).normalize_or_zero();

    let (life_min, life_max) = ordered_pair(
        tuning.thunder_lifetime_ticks_min,
        tuning.thunder_lifetime_ticks_max,
        1.0,
    );
    let (width_min, width_max) =
        ordered_pair(tuning.thunder_width_min, tuning.thunder_width_max, 0.1);

    let lifetime_ticks = rng.gen_range(life_min..life_max);
    let width = rng.gen_range(width_min..width_max);

    spawn_joint_entity(
        commands,
        meshes,
        materials,
        SpawnJointDesc {
            kind: JointKind::ThunderArc,
            texture: &assets.thunder_texture,
            position: start,
            start_position: start,
            target_position: end,
            direction,
            lifetime_ticks,
            tail_max: tuning.thunder_tail_max,
            light: Vec3::new(0.85, 0.92, 1.0),
            base_width: width,
            emissive: tuning.thunder_emissive,
            uv_scroll_speed: tuning.thunder_uv_scroll_speed,
            seed,
            alpha_mode: AlphaMode::Blend,
        },
    );
}

fn spawn_combo_burst(
    commands: &mut Commands,
    assets: &FxAssets,
    materials: &mut Assets<StandardMaterial>,
    tuning: &Skill43Tuning,
    center: Vec3,
) {
    let color = Vec3::new(1.0, 0.9, 0.8);
    let material = make_effect_material(
        materials,
        &assets.combo_texture,
        color,
        tuning.combo_emissive,
        1.0,
        AlphaMode::Add,
    );

    commands.spawn((
        Mesh3d(assets.quad_mesh.clone()),
        MeshMaterial3d(material.clone()),
        Transform::from_translation(center).with_scale(Vec3::new(
            tuning.combo_scale_start,
            tuning.combo_scale_start,
            1.0,
        )),
        NotShadowCaster,
        NotShadowReceiver,
        ComboSprite {
            material,
            lifetime: 0.0,
            max_lifetime: tuning.combo_lifetime_seconds,
            scale_start: tuning.combo_scale_start,
            scale_end: tuning.combo_scale_end,
            emissive_strength: tuning.combo_emissive,
            color,
        },
    ));
}

struct SpawnJointDesc<'a> {
    kind: JointKind,
    texture: &'a Handle<Image>,
    position: Vec3,
    start_position: Vec3,
    target_position: Vec3,
    direction: Vec3,
    lifetime_ticks: f32,
    tail_max: usize,
    light: Vec3,
    base_width: f32,
    emissive: f32,
    uv_scroll_speed: f32,
    seed: i32,
    alpha_mode: AlphaMode,
}

fn spawn_joint_entity(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    desc: SpawnJointDesc,
) {
    let mesh = meshes.add(empty_joint_mesh());
    let material = make_effect_material(
        materials,
        desc.texture,
        desc.light,
        desc.emissive,
        1.0,
        desc.alpha_mode,
    );

    let mut tails = VecDeque::new();
    tails.push_front(create_tail_frame(
        desc.position,
        desc.base_width,
        desc.direction,
        desc.kind != JointKind::ThunderArc,
    ));

    commands.spawn((
        Mesh3d(mesh.clone()),
        MeshMaterial3d(material.clone()),
        Transform::IDENTITY,
        NotShadowCaster,
        NotShadowReceiver,
        Skill43Joint {
            kind: desc.kind,
            lifetime_ticks: desc.lifetime_ticks,
            max_lifetime_ticks: desc.lifetime_ticks,
            tail_max: desc.tail_max,
            position: desc.position,
            start_position: desc.start_position,
            target_position: desc.target_position,
            direction: desc.direction,
            light: desc.light,
            base_width: desc.base_width,
            seed: desc.seed,
            uv_scroll_speed: desc.uv_scroll_speed,
            tails,
        },
        JointRender {
            mesh,
            material,
            emissive_strength: desc.emissive,
        },
    ));
}

fn update_skill43_joints(
    tuning: Res<Skill43Tuning>,
    state: Res<Skill43State>,
    mut joints: Query<&mut Skill43Joint>,
) {
    if state.frame_ticks == 0 {
        return;
    }

    for _ in 0..state.frame_ticks {
        for mut joint in &mut joints {
            match joint.kind {
                JointKind::RearSubType2 => update_rear_subtype2_joint(
                    &mut joint,
                    &tuning,
                    state.move_scene_frame,
                    state.current_sword_tip,
                ),
                JointKind::ForwardSpear => update_forward_spear_joint(&mut joint, &tuning, 1.0),
                JointKind::ThunderArc => {
                    update_thunder_arc_joint(&mut joint, state.move_scene_frame)
                }
            }

            joint.lifetime_ticks -= 1.0;
        }
    }
}

fn update_joint_meshes_and_materials(
    mut commands: Commands,
    state: Res<Skill43State>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    joints: Query<(Entity, &Skill43Joint, &JointRender)>,
) {
    for (entity, joint, render) in &joints {
        let alpha = (joint.lifetime_ticks / joint.max_lifetime_ticks).clamp(0.0, 1.0);
        let color_factor = match joint.kind {
            JointKind::RearSubType2 => (joint.lifetime_ticks.min(20.0) * 0.05).clamp(0.0, 1.0),
            JointKind::ForwardSpear | JointKind::ThunderArc => alpha,
        };

        if let Some(material) = materials.get_mut(&render.material) {
            material.base_color = Color::srgba(
                joint.light.x * color_factor,
                joint.light.y * color_factor,
                joint.light.z * color_factor,
                1.0,
            );
            material.emissive = LinearRgba::rgb(
                joint.light.x * render.emissive_strength * color_factor * 0.08,
                joint.light.y * render.emissive_strength * color_factor * 0.08,
                joint.light.z * render.emissive_strength * color_factor * 0.08,
            );
        }

        if let Some(mesh) = meshes.get_mut(&render.mesh) {
            *mesh = build_joint_mesh(joint, state.move_scene_frame as f32);
        }

        if joint.lifetime_ticks <= 0.0 {
            commands.entity(entity).despawn();
        }
    }
}

fn update_combo_sprites(
    mut commands: Commands,
    time: Res<Time>,
    tuning: Res<Skill43Tuning>,
    cameras: Query<&GlobalTransform, With<MainCamera>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut sprites: Query<(Entity, &mut ComboSprite, &mut Transform)>,
) {
    let camera_position = cameras
        .single()
        .map(|transform| transform.translation())
        .unwrap_or(Vec3::new(0.0, 500.0, 900.0));

    let dt = simulation_delta_seconds(&time, &tuning);

    for (entity, mut sprite, mut transform) in &mut sprites {
        sprite.lifetime += dt;
        let t = (sprite.lifetime / sprite.max_lifetime).clamp(0.0, 1.0);

        let scale = lerp(sprite.scale_start, sprite.scale_end, t);
        transform.scale = Vec3::new(scale, scale, 1.0);

        let mut rotation = Transform::from_translation(transform.translation)
            .looking_at(camera_position, Vec3::Y)
            .rotation;
        rotation *= Quat::from_rotation_y(PI);
        transform.rotation = rotation;

        if let Some(material) = materials.get_mut(&sprite.material) {
            let alpha = (1.0 - t).clamp(0.0, 1.0);
            material.base_color =
                Color::srgba(sprite.color.x, sprite.color.y, sprite.color.z, alpha);
            material.emissive = LinearRgba::rgb(
                sprite.color.x * sprite.emissive_strength * alpha,
                sprite.color.y * sprite.emissive_strength * alpha,
                sprite.color.z * sprite.emissive_strength * alpha,
            );
        }

        if sprite.lifetime >= sprite.max_lifetime {
            commands.entity(entity).despawn();
        }
    }
}

fn update_rear_subtype2_joint(
    joint: &mut Skill43Joint,
    tuning: &Skill43Tuning,
    move_scene_frame: i32,
    current_sword_tip: Vec3,
) {
    let life = joint.lifetime_ticks.max(0.0);
    let rate1 = ((life - 10.0) / 10.0).clamp(0.0, 1.0);
    let rate2 = 1.0 - rate1;

    let screw = get_magic_screw(
        joint.seed.wrapping_mul(17_721),
        move_scene_frame,
        tuning.rear_magic_speed_rate,
    ) * tuning.rear_magic_scale;

    let magic_position = joint.target_position + screw;
    joint.position = current_sword_tip * rate2 + magic_position * rate1;

    let width = (life * 3.0 * tuning.rear_width_scale).max(4.0);
    push_tail_sample(joint, joint.position, width);
}

fn update_forward_spear_joint(joint: &mut Skill43Joint, tuning: &Skill43Tuning, frame_delta: f32) {
    joint.position += joint.direction * tuning.forward_speed_per_tick * frame_delta;

    let life_ratio = (joint.lifetime_ticks / joint.max_lifetime_ticks).clamp(0.0, 1.0);
    let width = tuning.forward_width * (0.4 + life_ratio * 0.6);
    push_tail_sample(joint, joint.position, width.max(1.0));
}

fn update_thunder_arc_joint(joint: &mut Skill43Joint, move_scene_frame: i32) {
    let progress = 1.0 - (joint.lifetime_ticks / joint.max_lifetime_ticks).clamp(0.0, 1.0);
    let mut center = joint.start_position.lerp(joint.target_position, progress);

    let mut dir = joint.target_position - joint.start_position;
    if dir.length_squared() <= f32::EPSILON {
        dir = Vec3::Z;
    } else {
        dir = dir.normalize();
    }

    let mut side = dir.cross(Vec3::Y);
    if side.length_squared() <= f32::EPSILON {
        side = dir.cross(Vec3::X);
    }
    if side.length_squared() <= f32::EPSILON {
        side = Vec3::X;
    } else {
        side = side.normalize();
    }

    let phase = joint.seed as f32 * 0.37 + move_scene_frame as f32 * 0.35;
    center += side * phase.sin() * joint.base_width * (1.0 - progress) * 0.65;

    joint.position = center;
    let width = lerp(joint.base_width, joint.base_width * 0.35, progress).max(2.0);
    push_tail_sample(joint, center, width);
}

fn push_tail_sample(joint: &mut Skill43Joint, center: Vec3, width: f32) {
    if let Some(last) = joint.tails.front() {
        let last_center =
            (last.corners[0] + last.corners[1] + last.corners[2] + last.corners[3]) * 0.25;
        if last_center.distance_squared(center) <= 0.25 {
            return;
        }
    }

    if joint.tails.len() >= joint.tail_max {
        joint.tails.pop_back();
    }

    joint.tails.push_front(create_tail_frame(
        center,
        width.max(0.1),
        joint.direction,
        joint.kind != JointKind::ThunderArc,
    ));
}

fn build_joint_mesh(joint: &Skill43Joint, move_scene_frame: f32) -> Mesh {
    if joint.tails.len() < 2 {
        return empty_joint_mesh();
    }

    let samples: Vec<TailSample> = joint.tails.iter().copied().collect();
    let segment_count = samples.len().saturating_sub(1);
    if segment_count == 0 {
        return empty_joint_mesh();
    }

    let mut positions: Vec<[f32; 3]> = Vec::with_capacity(segment_count * 12);
    let mut normals: Vec<[f32; 3]> = Vec::with_capacity(segment_count * 12);
    let mut uvs: Vec<[f32; 2]> = Vec::with_capacity(segment_count * 12);
    let mut colors: Vec<[f32; 4]> = Vec::with_capacity(segment_count * 12);

    let num_tails = (samples.len().saturating_sub(1)) as f32;
    let max_tails = joint.tail_max.max(2) as f32;
    let scroll = (move_scene_frame * 0.016) % 1.0;

    for j in 0..segment_count {
        let current = samples[j];
        let next = samples[j + 1];

        let mut l1 = (num_tails - j as f32) / (max_tails - 1.0);
        let mut l2 = (num_tails - (j as f32 + 1.0)) / (max_tails - 1.0);

        if joint.kind == JointKind::ThunderArc {
            l1 = l1 * 2.0 - scroll * joint.uv_scroll_speed;
            l2 = l2 * 2.0 - scroll * joint.uv_scroll_speed;
        }
        let vertex_color = [1.0, 1.0, 1.0, 1.0];

        append_quad(
            &mut positions,
            &mut normals,
            &mut uvs,
            &mut colors,
            current.corners[2],
            current.corners[3],
            next.corners[3],
            next.corners[2],
            l1,
            l2,
            vertex_color,
        );

        let mut back_l1 = l1;
        let mut back_l2 = l2;
        if joint.kind == JointKind::ThunderArc {
            back_l1 += scroll * joint.uv_scroll_speed * 2.0;
            back_l2 += scroll * joint.uv_scroll_speed * 2.0;
        }

        append_quad(
            &mut positions,
            &mut normals,
            &mut uvs,
            &mut colors,
            current.corners[0],
            current.corners[1],
            next.corners[1],
            next.corners[0],
            back_l1,
            back_l2,
            vertex_color,
        );
    }

    if positions.is_empty() {
        return empty_joint_mesh();
    }

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh
}

fn create_tail_frame(position: Vec3, scale: f32, direction: Vec3, planar: bool) -> TailSample {
    let mut dir = direction;
    if dir.length_squared() <= f32::EPSILON {
        dir = Vec3::NEG_Z;
    }
    if planar {
        dir.y = 0.0;
        if dir.length_squared() <= f32::EPSILON {
            dir = Vec3::NEG_Z;
        }
    }
    dir = dir.normalize_or_zero();
    if dir.length_squared() <= f32::EPSILON {
        dir = Vec3::NEG_Z;
    }

    let mut right = if planar {
        Vec3::new(-dir.z, 0.0, dir.x)
    } else {
        dir.cross(Vec3::Y)
    };
    if right.length_squared() <= f32::EPSILON {
        right = Vec3::X;
    } else {
        right = right.normalize();
    }

    let up = Vec3::Y;
    let half = scale * 0.5;

    TailSample {
        corners: [
            position - right * half,
            position + right * half,
            position - up * half,
            position + up * half,
        ],
    }
}

fn append_quad(
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    uvs: &mut Vec<[f32; 2]>,
    colors: &mut Vec<[f32; 4]>,
    a: Vec3,
    b: Vec3,
    c: Vec3,
    d: Vec3,
    l1: f32,
    l2: f32,
    color: [f32; 4],
) {
    let normal = (b - a).cross(c - a).normalize_or_zero();
    let n: [f32; 3] = normal.into();

    positions.extend_from_slice(&[a.into(), b.into(), c.into(), a.into(), c.into(), d.into()]);
    normals.extend_from_slice(&[n, n, n, n, n, n]);
    uvs.extend_from_slice(&[
        [l1, 1.0],
        [l1, 0.0],
        [l2, 0.0],
        [l1, 1.0],
        [l2, 0.0],
        [l2, 1.0],
    ]);
    colors.extend_from_slice(&[color, color, color, color, color, color]);
}

fn empty_joint_mesh() -> Mesh {
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, Vec::<[f32; 3]>::new());
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, Vec::<[f32; 3]>::new());
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, Vec::<[f32; 2]>::new());
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, Vec::<[f32; 4]>::new());
    mesh
}

fn make_effect_material(
    materials: &mut Assets<StandardMaterial>,
    texture: &Handle<Image>,
    color: Vec3,
    emissive: f32,
    alpha: f32,
    alpha_mode: AlphaMode,
) -> Handle<StandardMaterial> {
    materials.add(StandardMaterial {
        base_color_texture: Some(texture.clone()),
        base_color: Color::srgba(color.x, color.y, color.z, alpha.clamp(0.0, 1.0)),
        emissive: LinearRgba::rgb(color.x * emissive, color.y * emissive, color.z * emissive),
        alpha_mode,
        unlit: true,
        double_sided: true,
        cull_mode: None,
        perceptual_roughness: 1.0,
        metallic: 0.0,
        reflectance: 0.0,
        ..default()
    })
}

fn update_camera_zoom(
    mut scroll_events: MessageReader<MouseWheel>,
    egui_wants_input: Res<EguiWantsInput>,
    tuning: Res<Skill43Tuning>,
    mut rig: ResMut<CameraRig>,
) {
    if egui_wants_input.wants_any_pointer_input() {
        return;
    }

    let mut scroll_delta = 0.0f32;
    for event in scroll_events.read() {
        scroll_delta += event.y;
    }
    if scroll_delta.abs() <= f32::EPSILON {
        return;
    }

    rig.distance =
        (rig.distance - scroll_delta * tuning.zoom_speed).clamp(tuning.zoom_min, tuning.zoom_max);
}

fn update_fps_overlay(time: Res<Time>, mut fps: ResMut<FpsOverlay>) {
    let dt = time.delta_secs();
    if dt <= f32::EPSILON {
        return;
    }

    fps.accumulator += dt;
    fps.frames = fps.frames.saturating_add(1);

    if fps.accumulator >= 0.25 {
        fps.value = fps.frames as f32 / fps.accumulator;
        fps.accumulator = 0.0;
        fps.frames = 0;
    }
}

fn apply_camera_rig(
    tuning: Res<Skill43Tuning>,
    mut rig: ResMut<CameraRig>,
    mut cameras: Query<&mut Transform, With<MainCamera>>,
) {
    let Ok(mut transform) = cameras.single_mut() else {
        return;
    };

    rig.distance = rig.distance.clamp(tuning.zoom_min, tuning.zoom_max);

    let look_at = Vec3::new(0.0, tuning.camera_look_height, 0.0);
    let pitch = tuning.camera_pitch_deg.to_radians();
    let yaw = tuning.camera_yaw_deg.to_radians();
    let horizontal = rig.distance * pitch.cos();
    let vertical = rig.distance * pitch.sin();
    let offset = Vec3::new(horizontal * yaw.sin(), vertical, horizontal * yaw.cos());
    let eye = look_at + offset;
    *transform = Transform::from_translation(eye).looking_at(look_at, Vec3::Y);
}

fn fit_scene_backdrop(
    windows: Query<&Window, With<PrimaryWindow>>,
    images: Res<Assets<Image>>,
    mut backdrops: Query<(&SceneBackdropImage, &mut Sprite, &mut Transform), With<SceneBackdrop>>,
) {
    let Ok(window) = windows.single() else {
        return;
    };

    for (image_ref, mut sprite, mut transform) in &mut backdrops {
        let Some(image) = images.get(&image_ref.handle) else {
            continue;
        };
        let size = image.size_f32();
        if size.x <= f32::EPSILON || size.y <= f32::EPSILON {
            continue;
        }

        let cover_scale = (window.width() / size.x).max(window.height() / size.y);
        sprite.custom_size = Some(Vec2::new(size.x, size.y));
        transform.scale = Vec3::new(cover_scale, cover_scale, 1.0);
        transform.translation = Vec3::new(0.0, 0.0, 0.0);
    }
}

fn draw_settings_window(
    mut contexts: EguiContexts,
    mut tuning: ResMut<Skill43Tuning>,
    mut rig: ResMut<CameraRig>,
    state: Res<Skill43State>,
    clock: Res<Skill43Clock>,
    mouse_target: Res<MouseTarget>,
) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };

    egui::Window::new("Skill 43 Config")
        .default_pos(egui::pos2(12.0, 12.0))
        .default_width(480.0)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Reset C++ Defaults").clicked() {
                    *tuning = Skill43Tuning::default();
                }
                if ui.button("Reset Zoom").clicked() {
                    rig.distance = tuning.camera_distance_default;
                }
            });

            ui.separator();
            ui.label("Runtime");
            ui.label(format!(
                "cast_state: {}",
                if let Some(cast) = state.active_cast {
                    format!("casting (AttackTime={})", cast.attack_time)
                } else {
                    "idle".to_string()
                }
            ));
            ui.label(format!(
                "impact_ticks_remaining: {}",
                state.impact_ticks_remaining
            ));
            ui.label(format!("move_scene_frame: {}", state.move_scene_frame));
            ui.label(format!("mu_ticks_this_frame: {}", state.frame_ticks));
            ui.label(format!("mu_pending_ticks: {}", clock.pending_ticks));
            ui.label(format!(
                "mouse_target: {} ({:.1}, {:.1}, {:.1})",
                if mouse_target.valid {
                    "valid"
                } else {
                    "invalid"
                },
                mouse_target.world.x,
                mouse_target.world.y,
                mouse_target.world.z
            ));

            ui.separator();
            ui.label("Global Timing");
            ui.label(format!(
                "MU tick base: {:.2} TPS ({:.3} s/tick)",
                MU_TICK_RATE, MU_TICK_SECONDS
            ));
            ui.add(
                egui::Slider::new(&mut tuning.animation_speed, 0.0..=3.0).text("animation_speed"),
            );
            ui.add(
                egui::Slider::new(&mut tuning.cast_plane_y, -300.0..=300.0).text("cast_plane_y"),
            );
            {
                let mut ticks = tuning.max_ticks_per_update as i32;
                ui.add(egui::Slider::new(&mut ticks, 1..=64).text("max_ticks_per_update"));
                tuning.max_ticks_per_update = ticks.max(1) as u32;
            }
            ui.add(
                egui::Slider::new(&mut tuning.limit_attack_time, 1..=120).text("limit_attack_time"),
            );

            ui.separator();
            ui.label("Rear Charge (MODEL_SPEARSKILL SubType 2)");
            ui.add(
                egui::Slider::new(&mut tuning.rear_phase_start, 1..=60).text("rear_phase_start"),
            );
            ui.add(egui::Slider::new(&mut tuning.rear_phase_end, 1..=60).text("rear_phase_end"));
            {
                let mut spawns = tuning.rear_spawns_per_tick as i32;
                ui.add(egui::Slider::new(&mut spawns, 0..=32).text("rear_spawns_per_tick"));
                tuning.rear_spawns_per_tick = spawns.max(0) as u32;
            }
            ui.add(
                egui::Slider::new(&mut tuning.rear_spawn_radius, 0.0..=2000.0)
                    .text("rear_spawn_radius"),
            );
            ui.add(
                egui::Slider::new(&mut tuning.rear_spawn_height, -300.0..=1000.0)
                    .text("rear_spawn_height"),
            );
            ui.add(
                egui::Slider::new(&mut tuning.rear_spawn_backward, 0.0..=5000.0)
                    .text("rear_spawn_backward"),
            );
            ui.add(
                egui::Slider::new(&mut tuning.sword_tip_height, -300.0..=800.0)
                    .text("sword_tip_height"),
            );
            ui.add(
                egui::Slider::new(&mut tuning.sword_tip_forward, 0.0..=2000.0)
                    .text("sword_tip_forward"),
            );
            ui.add(
                egui::Slider::new(&mut tuning.rear_lifetime_ticks, 1.0..=80.0)
                    .text("rear_lifetime_ticks"),
            );
            {
                let mut tail = tuning.rear_tail_max as i32;
                ui.add(egui::Slider::new(&mut tail, 2..=64).text("rear_tail_max"));
                tuning.rear_tail_max = tail.max(2) as usize;
            }
            ui.add(
                egui::Slider::new(&mut tuning.rear_magic_scale, 0.0..=1500.0)
                    .text("rear_magic_scale"),
            );
            ui.add(
                egui::Slider::new(&mut tuning.rear_magic_speed_rate, 0.1..=4.0)
                    .text("rear_magic_speed_rate"),
            );
            ui.add(
                egui::Slider::new(&mut tuning.rear_width_scale, 0.1..=3.0).text("rear_width_scale"),
            );
            ui.add(egui::Slider::new(&mut tuning.rear_emissive, 0.0..=40.0).text("rear_emissive"));

            ui.separator();
            ui.label("Forward Thrust (MODEL_SPEAR phase)");
            ui.add(
                egui::Slider::new(&mut tuning.forward_phase_start, 1..=60)
                    .text("forward_phase_start"),
            );
            ui.add(
                egui::Slider::new(&mut tuning.forward_phase_end, 1..=60).text("forward_phase_end"),
            );
            ui.add(
                egui::Slider::new(&mut tuning.forward_distance_base, -600.0..=2000.0)
                    .text("forward_distance_base"),
            );
            ui.add(
                egui::Slider::new(&mut tuning.forward_distance_step, -80.0..=80.0)
                    .text("forward_distance_step"),
            );
            ui.add(
                egui::Slider::new(&mut tuning.forward_distance_time_pivot, -60..=120)
                    .text("forward_distance_time_pivot"),
            );
            ui.add(
                egui::Slider::new(&mut tuning.forward_lifetime_ticks, 1.0..=60.0)
                    .text("forward_lifetime_ticks"),
            );
            {
                let mut tail = tuning.forward_tail_max as i32;
                ui.add(egui::Slider::new(&mut tail, 2..=64).text("forward_tail_max"));
                tuning.forward_tail_max = tail.max(2) as usize;
            }
            ui.add(
                egui::Slider::new(&mut tuning.forward_speed_per_tick, 0.0..=60.0)
                    .text("forward_speed_per_tick"),
            );
            ui.add(egui::Slider::new(&mut tuning.forward_width, 1.0..=120.0).text("forward_width"));
            ui.add(
                egui::Slider::new(&mut tuning.forward_emissive, 0.0..=40.0)
                    .text("forward_emissive"),
            );

            ui.separator();
            ui.label("Impact + Thunder Chain");
            ui.add(
                egui::Slider::new(&mut tuning.impact_start_attack_time, 1..=120)
                    .text("impact_start_attack_time"),
            );
            ui.add(
                egui::Slider::new(&mut tuning.impact_total_ticks, 1..=600)
                    .text("impact_total_ticks"),
            );
            {
                let mut arcs = tuning.impact_arcs_per_tick as i32;
                ui.add(egui::Slider::new(&mut arcs, 1..=64).text("impact_arcs_per_tick"));
                tuning.impact_arcs_per_tick = arcs.max(1) as u32;
            }
            ui.add(
                egui::Slider::new(&mut tuning.impact_center_height, -300.0..=1200.0)
                    .text("impact_center_height"),
            );
            ui.add(
                egui::Slider::new(&mut tuning.impact_offset_x, 0.0..=500.0).text("impact_offset_x"),
            );
            ui.add(
                egui::Slider::new(&mut tuning.impact_offset_y_min, -500.0..=500.0)
                    .text("impact_offset_y_min"),
            );
            ui.add(
                egui::Slider::new(&mut tuning.impact_offset_y_max, -500.0..=500.0)
                    .text("impact_offset_y_max"),
            );
            ui.add(
                egui::Slider::new(&mut tuning.impact_offset_z, 0.0..=500.0).text("impact_offset_z"),
            );
            ui.add(
                egui::Slider::new(&mut tuning.thunder_lifetime_ticks_min, 1.0..=60.0)
                    .text("thunder_lifetime_ticks_min"),
            );
            ui.add(
                egui::Slider::new(&mut tuning.thunder_lifetime_ticks_max, 2.0..=80.0)
                    .text("thunder_lifetime_ticks_max"),
            );
            {
                let mut tail = tuning.thunder_tail_max as i32;
                ui.add(egui::Slider::new(&mut tail, 2..=64).text("thunder_tail_max"));
                tuning.thunder_tail_max = tail.max(2) as usize;
            }
            ui.add(
                egui::Slider::new(&mut tuning.thunder_width_min, 0.1..=80.0)
                    .text("thunder_width_min"),
            );
            ui.add(
                egui::Slider::new(&mut tuning.thunder_width_max, 0.2..=120.0)
                    .text("thunder_width_max"),
            );
            ui.add(
                egui::Slider::new(&mut tuning.thunder_uv_scroll_speed, 0.0..=8.0)
                    .text("thunder_uv_scroll_speed"),
            );
            ui.add(
                egui::Slider::new(&mut tuning.thunder_emissive, 0.0..=40.0)
                    .text("thunder_emissive"),
            );

            ui.separator();
            ui.label("Combo Burst");
            ui.add(
                egui::Slider::new(&mut tuning.combo_lifetime_seconds, 0.01..=2.0)
                    .text("combo_lifetime_seconds"),
            );
            ui.add(
                egui::Slider::new(&mut tuning.combo_scale_start, 1.0..=500.0)
                    .text("combo_scale_start"),
            );
            ui.add(
                egui::Slider::new(&mut tuning.combo_scale_end, 1.0..=800.0).text("combo_scale_end"),
            );
            ui.add(
                egui::Slider::new(&mut tuning.combo_emissive, 0.0..=40.0).text("combo_emissive"),
            );

            ui.separator();
            ui.label("Camera (MU Fixed)");
            ui.add(
                egui::Slider::new(&mut tuning.camera_pitch_deg, 0.0..=89.0)
                    .text("camera_pitch_deg"),
            );
            ui.add(
                egui::Slider::new(&mut tuning.camera_yaw_deg, -180.0..=180.0)
                    .text("camera_yaw_deg"),
            );
            ui.add(
                egui::Slider::new(&mut tuning.camera_look_height, -300.0..=1500.0)
                    .text("camera_look_height"),
            );
            ui.add(
                egui::Slider::new(&mut tuning.camera_distance_default, 100.0..=5000.0)
                    .text("camera_distance_default"),
            );
            ui.add(egui::Slider::new(&mut tuning.zoom_min, 10.0..=5000.0).text("zoom_min"));
            ui.add(egui::Slider::new(&mut tuning.zoom_max, 20.0..=6000.0).text("zoom_max"));
            ui.add(egui::Slider::new(&mut tuning.zoom_speed, 0.0..=400.0).text("zoom_speed"));

            ui.separator();
            ui.label("RMB: cast | Scroll: zoom");
        });

    tuning.sanitize();
    rig.distance = rig.distance.clamp(tuning.zoom_min, tuning.zoom_max);
}

fn draw_fps_overlay(mut contexts: EguiContexts, fps: Res<FpsOverlay>) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };

    egui::Area::new(egui::Id::new("fps_overlay_top_right"))
        .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-12.0, 10.0))
        .interactable(false)
        .show(ctx, |ui| {
            ui.label(
                egui::RichText::new(format!("FPS: {:.1}", fps.value))
                    .color(egui::Color32::YELLOW)
                    .strong()
                    .monospace(),
            );
        });
}

fn sword_tip_anchor(caster: Vec3, forward: Vec3, tuning: &Skill43Tuning) -> Vec3 {
    caster + Vec3::new(0.0, tuning.sword_tip_height, 0.0) + forward * tuning.sword_tip_forward
}

fn random_impact_offset(rng: &mut rand::rngs::ThreadRng, tuning: &Skill43Tuning) -> Vec3 {
    let (y_min, y_max) = ordered_pair(tuning.impact_offset_y_min, tuning.impact_offset_y_max, 0.01);
    Vec3::new(
        rng.gen_range(-tuning.impact_offset_x..=tuning.impact_offset_x),
        rng.gen_range(y_min..=y_max),
        rng.gen_range(-tuning.impact_offset_z..=tuning.impact_offset_z),
    )
}

fn get_magic_screw(i_param: i32, move_scene_frame: i32, speed_rate: f32) -> Vec3 {
    let i_param = i_param.wrapping_add(move_scene_frame);

    let speed0 = 0.048 * speed_rate;
    let speed1 = 0.0613 * speed_rate;
    let speed2 = 0.1113 * speed_rate;

    let dir0 = ((i_param + 55_555) as f32 * speed0).sin() * (i_param as f32 * speed1).cos();
    let dir1 = ((i_param + 55_555) as f32 * speed0).sin() * (i_param as f32 * speed1).sin();
    let dir2 = ((i_param + 55_555) as f32 * speed0).cos();

    let sin_add = ((i_param + 11_111) as f32 * speed2).sin();
    let cos_add = ((i_param + 11_111) as f32 * speed2).cos();

    let cxx_x = cos_add * dir1 - sin_add * dir2;
    let cxx_y = sin_add * dir1 + cos_add * dir2;
    let cxx_z = dir0;

    Vec3::new(cxx_x, cxx_z, cxx_y)
}

fn next_seed(state: &mut Skill43State) -> i32 {
    let seed = state.next_seed;
    state.next_seed = state.next_seed.wrapping_add(1);
    seed
}

fn ordered_pair(a: f32, b: f32, epsilon: f32) -> (f32, f32) {
    let min = a.min(b);
    let max = a.max(b);
    if max - min < epsilon {
        (min, min + epsilon)
    } else {
        (min, max)
    }
}

fn simulation_delta_seconds(time: &Time, tuning: &Skill43Tuning) -> f32 {
    time.delta_secs() * tuning.animation_speed
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}
