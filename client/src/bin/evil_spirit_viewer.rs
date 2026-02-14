use bevy::asset::AssetPlugin;
use bevy::camera::ClearColorConfig;
use bevy::color::LinearRgba;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::input::mouse::MouseWheel;
use bevy::light::{NotShadowCaster, NotShadowReceiver};
use bevy::prelude::*;
use bevy::sprite::Anchor;
use bevy::window::{PrimaryWindow, WindowResolution};
use bevy_egui::input::EguiWantsInput;
use bevy_egui::{egui, EguiContexts, EguiPlugin, EguiPrimaryContextPass};
use rand::Rng;
use std::f32::consts::PI;
use std::path::Path;

/*
Skill Evil Spirit - Viewer fiel (C++ -> Rust)
=============================================

Objetivo:
- Recriar exclusivamente o visual do Evil Spirit em um bin isolado.
- Seguir o padrao de simulacao por tick MU (25 TPS), independente do FPS de render.
- Reproduzir o comportamento "2D em mundo 3D" com sprites/quads additive.

Referencias C++ principais:
- Disparo do skill e criacao dos 8 joints (4 angulos x escalas 80/20):
  cpp/MuClient5.2/source/ZzzCharacter.cpp (AT_SKILL_EVIL, ~4400..4428)
- Inicializacao do BITMAP_JOINT_SPIRIT subtipo 0:
  cpp/MuClient5.2/source/ZzzEffectJoint.cpp (~592..607)
  (Velocity=70, LifeTime=49, MaxTails=6, RenderType additive-like)
- Atualizacao do joint subtipo 0:
  cpp/MuClient5.2/source/ZzzEffectJoint.cpp (~3760..3833)
  (MoveHumming, jitter angular, clamp vertical)
- Funcoes de steering:
  cpp/MuClient5.2/source/ZzzAI.cpp (CreateAngle, TurnAngle2, MoveHumming)
- Base temporal de 25 FPS logicos:
  cpp/MuClient5.2/source/ZzzScene.cpp (passo fixo de 40ms)
*/

const REQUIRED_ASSET_SPIRIT_TEX: &str = "data/effect/joint_spirit_01.png";
const REQUIRED_ASSET_SHOCK_TEX: &str = "data/effect/joint_laser_01.png";
const REQUIRED_ASSET_GLOW_TEX: &str = "data/effect/flare_blue.png";
const REQUIRED_ASSET_BACKGROUND_TEX: &str = "prototype/noria-placeholder.png";

const MU_TICK_RATE: f32 = 25.0;
const MU_TICK_SECONDS: f32 = 1.0 / MU_TICK_RATE;
const SKILL_VFX_ALPHA_MODE: AlphaMode = AlphaMode::Add;

const EVIL_SPIRIT_BOLT_COUNT: usize = 8;
const EVIL_SPIRIT_MAX_TAILS: usize = 6;

#[derive(Resource)]
struct FxAssets {
    quad_mesh: Handle<Mesh>,
    shock_texture: Handle<Image>,
    glow_texture: Handle<Image>,
}

#[derive(Resource, Default)]
struct FpsOverlay {
    accumulator: f32,
    frames: u32,
    value: f32,
}

#[derive(Resource, Default)]
struct EvilSpiritClock {
    accumulator_seconds: f32,
    pending_ticks: u32,
}

#[derive(Resource)]
struct CameraRig {
    distance: f32,
}

#[derive(Resource, Default)]
struct ViewerCommands {
    trigger_cast: bool,
    stop_cast: bool,
    reset_defaults: bool,
}

#[derive(Clone, Copy)]
struct SpiritBoltState {
    active: bool,
    position: Vec3,
    tails: [Vec3; EVIL_SPIRIT_MAX_TAILS + 1],
    num_tails: usize,
    lifetime_ticks: i32,
    angle_pitch: f32,
    angle_yaw: f32,
    direction_pitch: f32,
    direction_yaw: f32,
    scale_cpp: f32,
}

impl Default for SpiritBoltState {
    fn default() -> Self {
        Self {
            active: false,
            position: Vec3::ZERO,
            tails: [Vec3::ZERO; EVIL_SPIRIT_MAX_TAILS + 1],
            num_tails: 0,
            lifetime_ticks: 0,
            angle_pitch: 0.0,
            angle_yaw: 0.0,
            direction_pitch: 0.0,
            direction_yaw: 0.0,
            scale_cpp: 20.0,
        }
    }
}

#[derive(Resource, Default)]
struct EvilSpiritRuntime {
    active: bool,
    bolts: [SpiritBoltState; EVIL_SPIRIT_BOLT_COUNT],
    move_scene_frame: i32,
    mu_ticks_this_frame: u32,
    autoplay_cooldown_ticks: i32,
    total_casts: u32,
}

#[derive(Resource, Clone)]
struct EvilSpiritTuning {
    animation_speed: f32,
    max_ticks_per_update: u32,
    autoplay: bool,
    autoplay_delay_ticks: i32,
    cast_plane_y: f32,
    anchor_x: f32,
    anchor_z: f32,
    target_height_offset: f32,
    spawn_height_offset: f32,
    life_ticks: i32,
    velocity_per_tick: f32,
    turn_per_tick: f32,
    jitter_pitch_scale: f32,
    jitter_yaw_scale: f32,
    damping_pitch: f32,
    damping_yaw: f32,
    lower_altitude_limit: f32,
    upper_altitude_limit: f32,
    lower_pitch_clamp: f32,
    upper_pitch_clamp: f32,
    tail_max: usize,
    core_size_factor: f32,
    tail_size_factor: f32,
    shock_size_factor: f32,
    shock_lifetime_ticks: f32,
    core_emissive: f32,
    tail_emissive: f32,
    shock_emissive: f32,
    core_alpha: f32,
    tail_alpha: f32,
    shock_alpha: f32,
    shock_enabled: bool,
    camera_pitch_deg: f32,
    camera_yaw_deg: f32,
    camera_look_height: f32,
    camera_distance_default: f32,
    zoom_min: f32,
    zoom_max: f32,
    zoom_speed: f32,
}

impl Default for EvilSpiritTuning {
    fn default() -> Self {
        Self {
            // MU: tempo logico relativo ao tick fixo.
            animation_speed: 1.0,
            max_ticks_per_update: 8,
            autoplay: true,
            autoplay_delay_ticks: 20,
            cast_plane_y: 0.0,
            anchor_x: 0.0,
            anchor_z: 0.0,
            // C++: TargetPosition[2] += 80; Position[2] += 100 no spawn.
            target_height_offset: 80.0,
            spawn_height_offset: 100.0,
            life_ticks: 49,
            velocity_per_tick: 70.0,
            turn_per_tick: 10.0,
            jitter_pitch_scale: 0.2,
            jitter_yaw_scale: 0.8,
            damping_pitch: 0.6,
            damping_yaw: 0.8,
            lower_altitude_limit: 100.0,
            upper_altitude_limit: 400.0,
            lower_pitch_clamp: -5.0,
            upper_pitch_clamp: 5.0,
            tail_max: EVIL_SPIRIT_MAX_TAILS,
            // Conversao de "Scale C++" (80/20) para tamanho de quad no viewer.
            core_size_factor: 1.8,
            tail_size_factor: 1.2,
            shock_size_factor: 2.1,
            shock_lifetime_ticks: 3.0,
            core_emissive: 10.0,
            tail_emissive: 8.0,
            shock_emissive: 16.0,
            core_alpha: 0.85,
            tail_alpha: 0.65,
            shock_alpha: 0.72,
            shock_enabled: true,
            camera_pitch_deg: 48.5,
            camera_yaw_deg: -45.0,
            camera_look_height: 90.0,
            camera_distance_default: 980.0,
            zoom_min: 280.0,
            zoom_max: 2400.0,
            zoom_speed: 90.0,
        }
    }
}

impl EvilSpiritTuning {
    fn sanitize(&mut self) {
        self.animation_speed = self.animation_speed.max(0.0);
        self.max_ticks_per_update = self.max_ticks_per_update.max(1);
        self.autoplay_delay_ticks = self.autoplay_delay_ticks.max(0);
        self.life_ticks = self.life_ticks.max(1);
        self.velocity_per_tick = self.velocity_per_tick.max(0.0);
        self.turn_per_tick = self.turn_per_tick.max(0.0);
        self.damping_pitch = self.damping_pitch.clamp(0.0, 1.0);
        self.damping_yaw = self.damping_yaw.clamp(0.0, 1.0);
        self.tail_max = self.tail_max.clamp(1, EVIL_SPIRIT_MAX_TAILS);
        self.core_size_factor = self.core_size_factor.max(0.01);
        self.tail_size_factor = self.tail_size_factor.max(0.01);
        self.shock_size_factor = self.shock_size_factor.max(0.01);
        self.shock_lifetime_ticks = self.shock_lifetime_ticks.max(0.1);
        self.core_emissive = self.core_emissive.max(0.0);
        self.tail_emissive = self.tail_emissive.max(0.0);
        self.shock_emissive = self.shock_emissive.max(0.0);
        self.core_alpha = self.core_alpha.clamp(0.0, 1.0);
        self.tail_alpha = self.tail_alpha.clamp(0.0, 1.0);
        self.shock_alpha = self.shock_alpha.clamp(0.0, 1.0);
        self.zoom_min = self.zoom_min.max(10.0);
        if self.zoom_max < self.zoom_min + 1.0 {
            self.zoom_max = self.zoom_min + 1.0;
        }
        self.zoom_speed = self.zoom_speed.max(0.0);
    }
}

#[derive(Component)]
struct MainCamera;

#[derive(Component)]
struct SceneBackdrop;

#[derive(Component)]
struct SceneBackdropImage {
    handle: Handle<Image>,
}

#[derive(Component)]
struct CoreSprite {
    bolt_index: usize,
    material: Handle<StandardMaterial>,
}

#[derive(Component)]
struct TailSprite {
    bolt_index: usize,
    tail_index: usize,
    material: Handle<StandardMaterial>,
}

#[derive(Component)]
struct ShockSprite {
    material: Handle<StandardMaterial>,
    lifetime_ticks: f32,
    max_lifetime_ticks: f32,
    base_size: f32,
}

fn main() {
    let asset_root = format!("{}/../assets", env!("CARGO_MANIFEST_DIR"));
    validate_required_assets(&asset_root);

    let tuning = EvilSpiritTuning::default();

    App::new()
        .insert_resource(ClearColor(Color::BLACK))
        .insert_resource(tuning.clone())
        .insert_resource(CameraRig {
            distance: tuning.camera_distance_default,
        })
        .insert_resource(FpsOverlay::default())
        .insert_resource(EvilSpiritClock::default())
        .insert_resource(EvilSpiritRuntime::default())
        .insert_resource(ViewerCommands::default())
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "evil_spirit_viewer - Skill Evil Spirit".to_string(),
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
                update_scene_frame,
                update_fps_overlay,
                handle_runtime_input,
                apply_viewer_commands,
                auto_cast_when_idle,
                simulate_evil_spirit,
                update_spirit_visuals,
                update_shock_sprites,
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
        REQUIRED_ASSET_SPIRIT_TEX,
        REQUIRED_ASSET_SHOCK_TEX,
        REQUIRED_ASSET_GLOW_TEX,
        REQUIRED_ASSET_BACKGROUND_TEX,
    ];

    let missing: Vec<&str> = required
        .iter()
        .copied()
        .filter(|relative| !Path::new(asset_root).join(relative).is_file())
        .collect();

    if !missing.is_empty() {
        panic!(
            "evil_spirit_viewer requires exact assets. Missing:\n{}",
            missing.join("\n")
        );
    }
}

fn setup_scene(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    tuning: Res<EvilSpiritTuning>,
) {
    let quad_mesh = meshes.add(Mesh::from(Rectangle::new(1.0, 1.0)));
    let background_texture = asset_server.load(REQUIRED_ASSET_BACKGROUND_TEX);
    let spirit_texture = asset_server.load(REQUIRED_ASSET_SPIRIT_TEX);
    let shock_texture = asset_server.load(REQUIRED_ASSET_SHOCK_TEX);
    let glow_texture = asset_server.load(REQUIRED_ASSET_GLOW_TEX);

    commands.insert_resource(FxAssets {
        quad_mesh: quad_mesh.clone(),
        shock_texture,
        glow_texture,
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
            illuminance: 5_000.0,
            shadows_enabled: false,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -1.0, 0.8, 0.0)),
    ));

    // Visual do spirit: 1 core + N tails fixos por bolt.
    for bolt_index in 0..EVIL_SPIRIT_BOLT_COUNT {
        let core_material = make_effect_material(
            &mut materials,
            &spirit_texture,
            Vec3::ONE,
            tuning.core_emissive,
            0.0,
            SKILL_VFX_ALPHA_MODE,
        );
        commands.spawn((
            Mesh3d(quad_mesh.clone()),
            MeshMaterial3d(core_material.clone()),
            Transform::default(),
            Visibility::Hidden,
            NotShadowCaster,
            NotShadowReceiver,
            CoreSprite {
                bolt_index,
                material: core_material,
            },
        ));

        for tail_index in 0..EVIL_SPIRIT_MAX_TAILS {
            let tail_material = make_effect_material(
                &mut materials,
                &spirit_texture,
                Vec3::new(0.6, 0.8, 1.0),
                tuning.tail_emissive,
                0.0,
                SKILL_VFX_ALPHA_MODE,
            );
            commands.spawn((
                Mesh3d(quad_mesh.clone()),
                MeshMaterial3d(tail_material.clone()),
                Transform::default(),
                Visibility::Hidden,
                NotShadowCaster,
                NotShadowReceiver,
                TailSprite {
                    bolt_index,
                    tail_index,
                    material: tail_material,
                },
            ));
        }
    }
}

fn update_scene_frame(
    time: Res<Time>,
    tuning: Res<EvilSpiritTuning>,
    mut clock: ResMut<EvilSpiritClock>,
    mut runtime: ResMut<EvilSpiritRuntime>,
) {
    let dt = simulation_delta_seconds(&time, &tuning);
    if dt > 0.0 {
        clock.accumulator_seconds += dt;
    }

    while clock.accumulator_seconds >= MU_TICK_SECONDS {
        clock.accumulator_seconds -= MU_TICK_SECONDS;
        clock.pending_ticks = clock.pending_ticks.saturating_add(1);
    }

    let ticks = clock.pending_ticks.min(tuning.max_ticks_per_update);
    clock.pending_ticks -= ticks;

    runtime.mu_ticks_this_frame = ticks;
    runtime.move_scene_frame = runtime.move_scene_frame.wrapping_add(ticks as i32);
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

fn handle_runtime_input(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    egui_wants_input: Res<EguiWantsInput>,
    mut viewer_cmds: ResMut<ViewerCommands>,
) {
    if egui_wants_input.wants_any_pointer_input() {
        return;
    }

    if mouse_buttons.just_pressed(MouseButton::Right) || keyboard.just_pressed(KeyCode::Space) {
        viewer_cmds.trigger_cast = true;
    }
}

fn apply_viewer_commands(
    mut commands: Commands,
    mut viewer_cmds: ResMut<ViewerCommands>,
    mut tuning: ResMut<EvilSpiritTuning>,
    mut runtime: ResMut<EvilSpiritRuntime>,
    mut rig: ResMut<CameraRig>,
    shocks: Query<Entity, With<ShockSprite>>,
) {
    if viewer_cmds.reset_defaults {
        *tuning = EvilSpiritTuning::default();
        rig.distance = tuning.camera_distance_default;
    }

    if viewer_cmds.stop_cast {
        runtime.active = false;
        for bolt in &mut runtime.bolts {
            bolt.active = false;
            bolt.num_tails = 0;
            bolt.lifetime_ticks = 0;
        }
        for entity in &shocks {
            commands.entity(entity).despawn();
        }
    }

    if viewer_cmds.trigger_cast {
        start_new_cast(&mut commands, &mut runtime, &tuning, &shocks);
    }

    viewer_cmds.trigger_cast = false;
    viewer_cmds.stop_cast = false;
    viewer_cmds.reset_defaults = false;
}

fn auto_cast_when_idle(
    mut commands: Commands,
    tuning: Res<EvilSpiritTuning>,
    mut runtime: ResMut<EvilSpiritRuntime>,
    shocks: Query<Entity, With<ShockSprite>>,
) {
    if !tuning.autoplay || runtime.active {
        return;
    }

    let ticks = runtime.mu_ticks_this_frame as i32;
    if ticks <= 0 {
        return;
    }

    runtime.autoplay_cooldown_ticks -= ticks;
    if runtime.autoplay_cooldown_ticks <= 0 {
        start_new_cast(&mut commands, &mut runtime, &tuning, &shocks);
    }
}

fn start_new_cast(
    commands: &mut Commands,
    runtime: &mut EvilSpiritRuntime,
    tuning: &EvilSpiritTuning,
    shocks: &Query<Entity, With<ShockSprite>>,
) {
    // C++ (ZzzCharacter.cpp): cria 4 angulos (0/90/180/270) e dois joints por angulo
    // com escala 80 e 20 (SubType 0), totalizando 8 bolts.
    let owner_pos = Vec3::new(tuning.anchor_x, tuning.cast_plane_y, tuning.anchor_z);
    let start_pos = owner_pos + Vec3::new(0.0, tuning.spawn_height_offset, 0.0);

    let mut write_index = 0usize;
    for ring in 0..4 {
        let base_angle = ring as f32 * 90.0;
        for scale_cpp in [80.0f32, 20.0f32] {
            runtime.bolts[write_index] = SpiritBoltState {
                active: true,
                position: start_pos,
                tails: [start_pos; EVIL_SPIRIT_MAX_TAILS + 1],
                num_tails: 0,
                lifetime_ticks: tuning.life_ticks,
                angle_pitch: 0.0,
                angle_yaw: base_angle,
                direction_pitch: 0.0,
                direction_yaw: 0.0,
                scale_cpp,
            };
            write_index += 1;
        }
    }

    runtime.active = true;
    runtime.total_casts = runtime.total_casts.saturating_add(1);
    runtime.autoplay_cooldown_ticks = tuning.autoplay_delay_ticks;

    for entity in shocks {
        commands.entity(entity).despawn();
    }
}

fn simulate_evil_spirit(
    mut commands: Commands,
    assets: Res<FxAssets>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    tuning: Res<EvilSpiritTuning>,
    mut runtime: ResMut<EvilSpiritRuntime>,
) {
    let ticks = runtime.mu_ticks_this_frame;
    if ticks == 0 || !runtime.active {
        return;
    }

    let target_pos = Vec3::new(
        tuning.anchor_x,
        tuning.cast_plane_y + tuning.target_height_offset,
        tuning.anchor_z,
    );
    let mut rng = rand::thread_rng();

    for _ in 0..ticks {
        let mut any_active = false;

        for bolt in &mut runtime.bolts {
            if !bolt.active {
                continue;
            }
            if bolt.lifetime_ticks <= 0 {
                bolt.active = false;
                continue;
            }

            any_active = true;

            if bolt.num_tails < tuning.tail_max {
                bolt.num_tails += 1;
            }
            for i in (1..=bolt.num_tails).rev() {
                bolt.tails[i] = bolt.tails[i - 1];
            }

            // C++ MoveJoint: deslocamento pelo Velocity ocorre antes do switch do subtipo.
            move_with_cpp_angles(
                &mut bolt.position,
                bolt.angle_pitch,
                bolt.angle_yaw,
                tuning.velocity_per_tick,
            );
            bolt.tails[0] = bolt.position;

            // C++ MoveHumming (ZzzAI.cpp) + jitter angular do subtipo 0.
            let _distance = move_humming(
                bolt.position,
                &mut bolt.angle_pitch,
                &mut bolt.angle_yaw,
                target_pos,
                tuning.turn_per_tick,
            );

            bolt.direction_pitch += (rng.gen_range(0..32) as f32 - 16.0) * tuning.jitter_pitch_scale;
            bolt.direction_yaw += (rng.gen_range(0..32) as f32 - 16.0) * tuning.jitter_yaw_scale;
            bolt.angle_pitch += bolt.direction_pitch;
            bolt.angle_yaw += bolt.direction_yaw;
            bolt.direction_pitch *= tuning.damping_pitch;
            bolt.direction_yaw *= tuning.damping_yaw;

            // C++ clamp de altura relativo ao terreno (aqui terreno fixo no cast_plane_y).
            if bolt.position.y < tuning.cast_plane_y + tuning.lower_altitude_limit {
                bolt.direction_pitch = 0.0;
                bolt.angle_pitch = tuning.lower_pitch_clamp;
            }
            if bolt.position.y > tuning.cast_plane_y + tuning.upper_altitude_limit {
                bolt.direction_pitch = 0.0;
                bolt.angle_pitch = tuning.upper_pitch_clamp;
            }

            // C++: para scale 80/subtype 0 cria MODEL_LASER continuamente.
            if tuning.shock_enabled && bolt.scale_cpp >= 79.0 {
                let jitter = Vec3::new(
                    rng.gen_range(-10.0f32..=10.0),
                    rng.gen_range(-8.0f32..=8.0),
                    rng.gen_range(-10.0f32..=10.0),
                );
                spawn_shock_sprite(
                    &mut commands,
                    &assets,
                    &mut materials,
                    bolt.position + jitter,
                    bolt.scale_cpp * tuning.shock_size_factor,
                    tuning.shock_lifetime_ticks,
                    tuning.shock_emissive,
                    tuning.shock_alpha,
                    rng.gen_range(0.0f32..1.0) < 0.35,
                );
            }

            bolt.lifetime_ticks -= 1;
            if bolt.lifetime_ticks <= 0 {
                bolt.active = false;
            }
        }

        runtime.active = any_active && runtime.bolts.iter().any(|bolt| bolt.active);
        if !runtime.active {
            runtime.autoplay_cooldown_ticks = tuning.autoplay_delay_ticks;
            break;
        }
    }
}

fn update_spirit_visuals(
    tuning: Res<EvilSpiritTuning>,
    runtime: Res<EvilSpiritRuntime>,
    camera: Query<&GlobalTransform, With<MainCamera>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut cores: Query<(&CoreSprite, &mut Transform, &mut Visibility)>,
    mut tails: Query<(&TailSprite, &mut Transform, &mut Visibility)>,
) {
    let Ok(camera_transform) = camera.single() else {
        return;
    };
    let camera_pos = camera_transform.translation();

    for (core, mut transform, mut visibility) in &mut cores {
        let bolt = runtime.bolts[core.bolt_index];
        if !bolt.active {
            *visibility = Visibility::Hidden;
            continue;
        }

        *visibility = Visibility::Visible;
        let life_norm = (bolt.lifetime_ticks as f32 / tuning.life_ticks as f32).clamp(0.0, 1.0);
        let pulse = 0.85 + 0.15 * ((runtime.move_scene_frame as f32 * 0.18).sin() + 1.0) * 0.5;
        let size = bolt.scale_cpp * tuning.core_size_factor * (0.65 + 0.35 * life_norm) * pulse;
        *transform = billboard_transform(bolt.position, camera_pos, size);

        if let Some(material) = materials.get_mut(&core.material) {
            let light = if bolt.scale_cpp >= 79.0 {
                Vec3::new(1.0, 1.0, 1.0)
            } else {
                Vec3::new(0.55, 0.78, 1.0)
            };
            material.base_color = Color::srgba(
                light.x,
                light.y,
                light.z,
                (tuning.core_alpha * life_norm).clamp(0.0, 1.0),
            );
            let emissive = tuning.core_emissive * (0.7 + 0.3 * life_norm);
            material.emissive = LinearRgba::rgb(light.x * emissive, light.y * emissive, light.z * emissive);
            material.alpha_mode = SKILL_VFX_ALPHA_MODE;
        }
    }

    for (tail, mut transform, mut visibility) in &mut tails {
        let bolt = runtime.bolts[tail.bolt_index];
        if !bolt.active || tail.tail_index >= bolt.num_tails {
            *visibility = Visibility::Hidden;
            continue;
        }

        let tail_pos = bolt.tails[tail.tail_index];
        let life_norm = (bolt.lifetime_ticks as f32 / tuning.life_ticks as f32).clamp(0.0, 1.0);
        let fade = 1.0 - (tail.tail_index as f32 / EVIL_SPIRIT_MAX_TAILS as f32);
        let size = bolt.scale_cpp * tuning.tail_size_factor * (0.5 + 0.6 * fade);
        *transform = billboard_transform(tail_pos, camera_pos, size);
        *visibility = Visibility::Visible;

        if let Some(material) = materials.get_mut(&tail.material) {
            let light = if bolt.scale_cpp >= 79.0 {
                Vec3::new(0.72, 0.88, 1.0)
            } else {
                Vec3::new(0.45, 0.72, 1.0)
            };
            let alpha = (tuning.tail_alpha * life_norm * fade).clamp(0.0, 1.0);
            material.base_color = Color::srgba(light.x, light.y, light.z, alpha);
            let emissive = tuning.tail_emissive * fade;
            material.emissive = LinearRgba::rgb(light.x * emissive, light.y * emissive, light.z * emissive);
            material.alpha_mode = SKILL_VFX_ALPHA_MODE;
        }
    }
}

fn update_shock_sprites(
    mut commands: Commands,
    tuning: Res<EvilSpiritTuning>,
    runtime: Res<EvilSpiritRuntime>,
    camera: Query<&GlobalTransform, With<MainCamera>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut shocks: Query<(Entity, &mut ShockSprite, &mut Transform, &mut Visibility)>,
) {
    let Ok(camera_transform) = camera.single() else {
        return;
    };
    let camera_pos = camera_transform.translation();
    let delta_ticks = runtime.mu_ticks_this_frame as f32;
    if delta_ticks <= 0.0 {
        return;
    }

    for (entity, mut shock, mut transform, mut visibility) in &mut shocks {
        shock.lifetime_ticks -= delta_ticks;
        if shock.lifetime_ticks <= 0.0 {
            commands.entity(entity).despawn();
            continue;
        }

        let t = 1.0 - (shock.lifetime_ticks / shock.max_lifetime_ticks).clamp(0.0, 1.0);
        let scale = lerp(1.0, 1.9, t);
        transform.scale = Vec3::new(shock.base_size * scale, shock.base_size * scale, 1.0);
        transform.rotation = billboard_transform(transform.translation, camera_pos, 1.0).rotation;
        *visibility = Visibility::Visible;

        if let Some(material) = materials.get_mut(&shock.material) {
            let alpha = (tuning.shock_alpha * (1.0 - t)).clamp(0.0, 1.0);
            let color = Vec3::new(0.80, 0.92, 1.0);
            material.base_color = Color::srgba(color.x, color.y, color.z, alpha);
            let emissive = tuning.shock_emissive * (1.0 - t * 0.65);
            material.emissive = LinearRgba::rgb(color.x * emissive, color.y * emissive, color.z * emissive);
            material.alpha_mode = SKILL_VFX_ALPHA_MODE;
        }
    }
}

fn spawn_shock_sprite(
    commands: &mut Commands,
    assets: &FxAssets,
    materials: &mut Assets<StandardMaterial>,
    position: Vec3,
    size: f32,
    lifetime_ticks: f32,
    emissive: f32,
    alpha: f32,
    use_glow_texture: bool,
) {
    let texture = if use_glow_texture {
        &assets.glow_texture
    } else {
        &assets.shock_texture
    };
    let material = make_effect_material(
        materials,
        texture,
        Vec3::new(0.8, 0.9, 1.0),
        emissive,
        alpha,
        SKILL_VFX_ALPHA_MODE,
    );

    commands.spawn((
        Mesh3d(assets.quad_mesh.clone()),
        MeshMaterial3d(material.clone()),
        Transform {
            translation: position,
            scale: Vec3::new(size, size, 1.0),
            ..default()
        },
        Visibility::Visible,
        NotShadowCaster,
        NotShadowReceiver,
        ShockSprite {
            material,
            lifetime_ticks,
            max_lifetime_ticks: lifetime_ticks,
            base_size: size,
        },
    ));
}

fn move_with_cpp_angles(position: &mut Vec3, angle_pitch: f32, angle_yaw: f32, velocity: f32) {
    // Aproximacao equivalente ao vetor local (0, -Velocity, 0) rotacionado por AngleMatrix C++.
    let yaw = angle_yaw.to_radians();
    let pitch = angle_pitch.to_radians();
    let horizontal = pitch.cos();
    let dir = Vec3::new(yaw.sin() * horizontal, pitch.sin(), -yaw.cos() * horizontal);
    *position += dir * velocity;
}

fn move_humming(
    position: Vec3,
    angle_pitch: &mut f32,
    angle_yaw: &mut f32,
    target_position: Vec3,
    turn: f32,
) -> f32 {
    let target_angle_h = create_angle(position.x, position.z, target_position.x, target_position.z);
    *angle_yaw = turn_angle2(*angle_yaw, target_angle_h, turn);

    let range = position - target_position;
    let horizontal_distance = (range.x * range.x + range.z * range.z).sqrt();
    let target_angle_v = 360.0 - create_angle(position.y, horizontal_distance, target_position.y, 0.0);
    *angle_pitch = turn_angle2(*angle_pitch, target_angle_v, turn);

    range.length()
}

// Port direto de CreateAngle (ZzzAI.cpp).
fn create_angle(x1: f32, y1: f32, x2: f32, y2: f32) -> f32 {
    let nx2 = x2 - x1;
    let ny2 = y2 - y1;
    if nx2.abs() < 0.0001 {
        if ny2 < 0.0 {
            return 0.0;
        }
        return 180.0;
    }
    if ny2.abs() < 0.0001 {
        if nx2 < 0.0 {
            return 270.0;
        }
        return 90.0;
    }

    let angle = (ny2 / nx2).atan() / PI * 180.0 + 90.0;
    if nx2 < 0.0 {
        angle + 180.0
    } else {
        angle
    }
}

// Port direto de TurnAngle2 (ZzzAI.cpp).
fn turn_angle2(mut angle: f32, mut target: f32, d: f32) -> f32 {
    if angle < 0.0 {
        angle += 360.0;
    }
    if target < 0.0 {
        target += 360.0;
    }

    if angle < 180.0 {
        let aa = angle - d;
        if target >= angle + d && target < angle + 180.0 {
            angle += d;
        } else if aa >= 0.0 && (target >= angle + 180.0 || target < aa) {
            angle -= d;
        } else if aa < 0.0 && (target >= angle + 180.0 && target < aa + 360.0) {
            angle = angle - d + 360.0;
        } else {
            angle = target;
        }
    } else {
        let aa = angle + d;
        if target < angle - d && target >= angle - 180.0 {
            angle -= d;
        } else if aa < 360.0 && (target < angle - 180.0 || target >= aa) {
            angle += d;
        } else if aa >= 360.0 && (target < angle - 180.0 && target >= aa - 360.0) {
            angle = angle + d - 360.0;
        } else {
            angle = target;
        }
    }

    angle
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

fn billboard_transform(position: Vec3, camera_position: Vec3, size: f32) -> Transform {
    let mut transform = Transform::from_translation(position).looking_at(camera_position, Vec3::Y);
    transform.scale = Vec3::new(size, size, 1.0);
    transform
}

fn update_camera_zoom(
    mut scroll_events: MessageReader<MouseWheel>,
    egui_wants_input: Res<EguiWantsInput>,
    tuning: Res<EvilSpiritTuning>,
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

fn apply_camera_rig(
    tuning: Res<EvilSpiritTuning>,
    mut rig: ResMut<CameraRig>,
    mut cameras: Query<&mut Transform, With<MainCamera>>,
) {
    let Ok(mut transform) = cameras.single_mut() else {
        return;
    };

    rig.distance = rig.distance.clamp(tuning.zoom_min, tuning.zoom_max);
    let look_at = Vec3::new(tuning.anchor_x, tuning.camera_look_height, tuning.anchor_z);
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
        transform.translation = Vec3::ZERO;
    }
}

fn draw_settings_window(
    mut contexts: EguiContexts,
    mut tuning: ResMut<EvilSpiritTuning>,
    mut rig: ResMut<CameraRig>,
    mut runtime: ResMut<EvilSpiritRuntime>,
    mut viewer_cmds: ResMut<ViewerCommands>,
    clock: Res<EvilSpiritClock>,
) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };

    egui::Window::new("Evil Spirit Config")
        .default_pos(egui::pos2(12.0, 12.0))
        .default_width(460.0)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Cast Now").clicked() {
                    viewer_cmds.trigger_cast = true;
                }
                if ui.button("Stop").clicked() {
                    viewer_cmds.stop_cast = true;
                }
                if ui.button("Reset C++ Defaults").clicked() {
                    viewer_cmds.reset_defaults = true;
                }
                if ui.button("Reset Zoom").clicked() {
                    rig.distance = tuning.camera_distance_default;
                }
            });

            ui.separator();
            let active_count = runtime.bolts.iter().filter(|bolt| bolt.active).count();
            ui.label(format!("casts: {}", runtime.total_casts));
            ui.label(format!("active: {} ({} bolts)", runtime.active, active_count));
            ui.label(format!("mu_ticks_this_frame: {}", runtime.mu_ticks_this_frame));
            ui.label(format!("mu_pending_ticks: {}", clock.pending_ticks));
            ui.label(format!("move_scene_frame: {}", runtime.move_scene_frame));
            ui.label(format!("autoplay_cooldown_ticks: {}", runtime.autoplay_cooldown_ticks));

            ui.separator();
            ui.label("Timing (MU fixed step)");
            ui.label(format!(
                "MU tick base: {:.2} TPS ({:.3} s/tick)",
                MU_TICK_RATE, MU_TICK_SECONDS
            ));
            ui.add(egui::Slider::new(&mut tuning.animation_speed, 0.0..=3.0).text("animation_speed"));
            {
                let mut max_ticks = tuning.max_ticks_per_update as i32;
                ui.add(egui::Slider::new(&mut max_ticks, 1..=64).text("max_ticks_per_update"));
                tuning.max_ticks_per_update = max_ticks.max(1) as u32;
            }
            ui.checkbox(&mut tuning.autoplay, "autoplay");
            ui.add(
                egui::Slider::new(&mut tuning.autoplay_delay_ticks, 0..=300)
                    .text("autoplay_delay_ticks"),
            );

            ui.separator();
            ui.label("C++ Joint(0) motion");
            ui.add(egui::Slider::new(&mut tuning.cast_plane_y, -300.0..=300.0).text("cast_plane_y"));
            ui.add(egui::Slider::new(&mut tuning.anchor_x, -1000.0..=1000.0).text("anchor_x"));
            ui.add(egui::Slider::new(&mut tuning.anchor_z, -1000.0..=1000.0).text("anchor_z"));
            ui.add(
                egui::Slider::new(&mut tuning.target_height_offset, -200.0..=500.0)
                    .text("target_height_offset"),
            );
            ui.add(
                egui::Slider::new(&mut tuning.spawn_height_offset, -200.0..=600.0)
                    .text("spawn_height_offset"),
            );
            ui.add(egui::Slider::new(&mut tuning.life_ticks, 1..=200).text("life_ticks"));
            ui.add(
                egui::Slider::new(&mut tuning.velocity_per_tick, 0.0..=150.0)
                    .text("velocity_per_tick"),
            );
            ui.add(egui::Slider::new(&mut tuning.turn_per_tick, 0.0..=50.0).text("turn_per_tick"));
            ui.add(
                egui::Slider::new(&mut tuning.jitter_pitch_scale, 0.0..=2.0)
                    .text("jitter_pitch_scale"),
            );
            ui.add(
                egui::Slider::new(&mut tuning.jitter_yaw_scale, 0.0..=2.0)
                    .text("jitter_yaw_scale"),
            );
            ui.add(egui::Slider::new(&mut tuning.damping_pitch, 0.0..=1.0).text("damping_pitch"));
            ui.add(egui::Slider::new(&mut tuning.damping_yaw, 0.0..=1.0).text("damping_yaw"));
            ui.add(
                egui::Slider::new(&mut tuning.lower_altitude_limit, 0.0..=500.0)
                    .text("lower_altitude_limit"),
            );
            ui.add(
                egui::Slider::new(&mut tuning.upper_altitude_limit, 100.0..=1000.0)
                    .text("upper_altitude_limit"),
            );
            ui.add(
                egui::Slider::new(&mut tuning.lower_pitch_clamp, -90.0..=90.0)
                    .text("lower_pitch_clamp"),
            );
            ui.add(
                egui::Slider::new(&mut tuning.upper_pitch_clamp, -90.0..=90.0)
                    .text("upper_pitch_clamp"),
            );
            {
                let mut tail_max = tuning.tail_max as i32;
                ui.add(egui::Slider::new(&mut tail_max, 1..=EVIL_SPIRIT_MAX_TAILS as i32).text("tail_max"));
                tuning.tail_max = tail_max.max(1) as usize;
            }

            ui.separator();
            ui.label("Visual / Additive");
            ui.add(egui::Slider::new(&mut tuning.core_size_factor, 0.1..=5.0).text("core_size_factor"));
            ui.add(egui::Slider::new(&mut tuning.tail_size_factor, 0.1..=5.0).text("tail_size_factor"));
            ui.add(egui::Slider::new(&mut tuning.core_alpha, 0.0..=1.0).text("core_alpha"));
            ui.add(egui::Slider::new(&mut tuning.tail_alpha, 0.0..=1.0).text("tail_alpha"));
            ui.add(egui::Slider::new(&mut tuning.core_emissive, 0.0..=40.0).text("core_emissive"));
            ui.add(egui::Slider::new(&mut tuning.tail_emissive, 0.0..=40.0).text("tail_emissive"));
            ui.checkbox(&mut tuning.shock_enabled, "shock_enabled");
            ui.add(egui::Slider::new(&mut tuning.shock_size_factor, 0.1..=6.0).text("shock_size_factor"));
            ui.add(
                egui::Slider::new(&mut tuning.shock_lifetime_ticks, 0.1..=12.0)
                    .text("shock_lifetime_ticks"),
            );
            ui.add(egui::Slider::new(&mut tuning.shock_alpha, 0.0..=1.0).text("shock_alpha"));
            ui.add(egui::Slider::new(&mut tuning.shock_emissive, 0.0..=60.0).text("shock_emissive"));

            ui.separator();
            ui.label("Camera");
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
            ui.label("RMB/Space: cast | Scroll: zoom");
        });

    tuning.sanitize();
    rig.distance = rig.distance.clamp(tuning.zoom_min, tuning.zoom_max);
    runtime.mu_ticks_this_frame = runtime.mu_ticks_this_frame.min(tuning.max_ticks_per_update);
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

fn simulation_delta_seconds(time: &Time, tuning: &EvilSpiritTuning) -> f32 {
    time.delta_secs() * tuning.animation_speed
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}
