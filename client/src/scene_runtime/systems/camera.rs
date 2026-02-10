use crate::bevy_compat::*;
use crate::scene_runtime::components::*;
use crate::scene_runtime::state::RuntimeSceneAssets;
use crate::scene_runtime::world_coordinates::{mirror_map_position_with_axis, world_mirror_axis};
use bevy::camera::Projection;
use bevy::input::mouse::{MouseMotion, MouseScrollUnit, MouseWheel};
use bevy::prelude::*;

/// Marker for camera tour setup
#[derive(Component)]
pub struct CameraTourSetup;

/// UI marker for debug free camera hint text.
#[derive(Component)]
pub struct DebugFreeCameraHint;

#[derive(Resource)]
pub struct DebugFreeCameraController {
    pub enabled: bool,
    pub move_speed: f32,
    pub look_sensitivity: f32,
    pub zoom_sensitivity: f32,
    pub yaw: f32,
    pub pitch: f32,
    pub tour_was_active: bool,
}

impl Default for DebugFreeCameraController {
    fn default() -> Self {
        Self {
            enabled: false,
            move_speed: 2_500.0,
            look_sensitivity: 0.0025,
            zoom_sensitivity: 350.0,
            yaw: 0.0,
            pitch: 0.0,
            tour_was_active: true,
        }
    }
}

fn login_camera_fov_for_world(world_name: &str) -> Option<f32> {
    match world_name {
        // Legacy login scene: C++ WD_55LOGINSCENE (assets from World56).
        "world_56" => Some(35.0_f32.to_radians()),
        // New login scene v1: C++ WD_73NEW_LOGIN_SCENE (assets from World74).
        "world_74" => Some(65.0_f32.to_radians()),
        // New login scene v2: C++ WD_77NEW_LOGIN_SCENE (assets from World78).
        "world_78" => Some(61.0_f32.to_radians()),
        _ => None,
    }
}

pub fn reset_debug_free_camera(mut controller: ResMut<DebugFreeCameraController>) {
    *controller = DebugFreeCameraController::default();
}

pub fn spawn_debug_free_camera_hint(mut commands: Commands) {
    let mut hint_text = TextBundle::from_section(
        "",
        TextStyle {
            font_size: 16.0,
            color: Color::WHITE,
            ..default()
        },
    )
    .with_style(Style {
        position_type: PositionType::Absolute,
        top: Val::Px(14.0),
        left: Val::Px(14.0),
        ..default()
    });
    hint_text.background_color = BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.5));

    commands.spawn((
        RuntimeSceneEntity,
        DebugOverlayElement,
        DebugFreeCameraHint,
        hint_text,
    ));
}

pub fn update_debug_free_camera_hint(
    controller: Res<DebugFreeCameraController>,
    mut hints: Query<&mut Text, With<DebugFreeCameraHint>>,
) {
    let mode = if controller.enabled { "ON" } else { "OFF" };
    for mut text in &mut hints {
        text.0 = format!(
            "[DEBUG] Ctrl+W: Free Camera {mode} | F3: Toggle HUD | WASD mover | Botao direito + mouse olhar | Scroll zoom"
        );
    }
}

pub fn toggle_debug_free_camera(
    keys: Res<ButtonInput<KeyCode>>,
    mut controller: ResMut<DebugFreeCameraController>,
    mut camera_query: Query<(&Transform, Option<&mut CameraTour>), With<Camera3d>>,
) {
    let ctrl_pressed = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    if !(ctrl_pressed && keys.just_pressed(KeyCode::KeyW)) {
        return;
    }

    let Ok((transform, maybe_tour)) = camera_query.single_mut() else {
        return;
    };

    controller.enabled = !controller.enabled;
    if controller.enabled {
        let forward = transform.forward();
        controller.yaw = forward.x.atan2(forward.z);
        controller.pitch = forward.y.clamp(-0.999, 0.999).asin();
    }

    if let Some(mut tour) = maybe_tour {
        if controller.enabled {
            controller.tour_was_active = tour.active;
            tour.active = false;
        } else {
            tour.active = controller.tour_was_active;
        }
    }

    info!(
        "Debug free camera {}",
        if controller.enabled {
            "enabled"
        } else {
            "disabled"
        }
    );
}

pub fn control_debug_free_camera(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut mouse_motion: MessageReader<MouseMotion>,
    mut mouse_wheel: MessageReader<MouseWheel>,
    mut controller: ResMut<DebugFreeCameraController>,
    mut camera_query: Query<&mut Transform, With<Camera3d>>,
) {
    if !controller.enabled {
        // Drain events while disabled so accumulated movement isn't applied on toggle.
        for _ in mouse_motion.read() {}
        for _ in mouse_wheel.read() {}
        return;
    }

    let Ok(mut transform) = camera_query.single_mut() else {
        return;
    };

    let mut mouse_delta = Vec2::ZERO;
    for motion in mouse_motion.read() {
        mouse_delta += motion.delta;
    }
    if mouse_buttons.pressed(MouseButton::Right) {
        controller.yaw -= mouse_delta.x * controller.look_sensitivity;
        controller.pitch =
            (controller.pitch - mouse_delta.y * controller.look_sensitivity).clamp(-1.54, 1.54);
        transform.rotation = Quat::from_euler(EulerRot::YXZ, controller.yaw, controller.pitch, 0.0);
    }

    let mut move_dir = Vec3::ZERO;
    if keys.pressed(KeyCode::KeyW) {
        move_dir += *transform.forward();
    }
    if keys.pressed(KeyCode::KeyS) {
        move_dir -= *transform.forward();
    }
    if keys.pressed(KeyCode::KeyA) {
        move_dir -= *transform.right();
    }
    if keys.pressed(KeyCode::KeyD) {
        move_dir += *transform.right();
    }
    if keys.pressed(KeyCode::Space) {
        move_dir += Vec3::Y;
    }
    if keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight) {
        move_dir -= Vec3::Y;
    }
    if move_dir.length_squared() > f32::EPSILON {
        let sprint = if keys.pressed(KeyCode::AltLeft) || keys.pressed(KeyCode::AltRight) {
            3.5
        } else {
            1.0
        };
        transform.translation +=
            move_dir.normalize() * controller.move_speed * sprint * time.delta_secs();
    }

    let mut zoom_units = 0.0;
    for wheel in mouse_wheel.read() {
        let unit_scale = match wheel.unit {
            MouseScrollUnit::Line => 1.0,
            MouseScrollUnit::Pixel => 0.03,
        };
        zoom_units += wheel.y * unit_scale;
    }
    if zoom_units.abs() > f32::EPSILON {
        let forward = *transform.forward();
        transform.translation += forward * zoom_units * controller.zoom_sensitivity;
    }
}

/// System to setup camera tour once assets are loaded
pub fn setup_camera_tour(
    mut commands: Commands,
    assets: Res<RuntimeSceneAssets>,
    terrain_configs: Res<Assets<TerrainConfig>>,
    camera_tour_data: Res<Assets<CameraTourData>>,
    mut camera_query: Query<(Entity, &mut Projection), (With<Camera3d>, Without<CameraTour>)>,
    setup_query: Query<&CameraTourSetup>,
) {
    // Only setup once
    if !setup_query.is_empty() {
        return;
    }

    // Wait for assets to be loaded
    if !assets.loaded {
        return;
    }

    let Some(world) = assets.world.as_ref() else {
        return;
    };

    let Some(terrain_config) = terrain_configs.get(&world.terrain_config) else {
        return;
    };

    let Some(tour_data) = camera_tour_data.get(&world.camera_tour) else {
        return;
    };

    let map_max_x =
        (terrain_config.size.width.saturating_sub(1) as f32) * terrain_config.size.scale;
    let map_max_z =
        (terrain_config.size.depth.saturating_sub(1) as f32) * terrain_config.size.scale;
    let mirror_axis = world_mirror_axis();

    info!(
        "Setting up camera tour with {} waypoints",
        tour_data.waypoints.len()
    );

    // Convert waypoint data to component format
    let waypoints: Vec<CameraWaypoint> = tour_data
        .waypoints
        .iter()
        .map(|wp| CameraWaypoint {
            position: mirror_map_position_with_axis(
                Vec3::from(wp.position),
                map_max_x,
                map_max_z,
                mirror_axis,
            ),
            look_at: mirror_map_position_with_axis(
                Vec3::from(wp.look_at),
                map_max_x,
                map_max_z,
                mirror_axis,
            ),
            move_acceleration: wp.move_acceleration,
            distance_level: wp.distance_level,
            delay: wp.delay,
        })
        .collect();

    if waypoints.is_empty() {
        warn!("Camera tour has no waypoints");
        return;
    }

    // Add camera tour to the main 3D camera
    if let Ok((camera_entity, mut projection)) = camera_query.single_mut() {
        if let Some(fov_radians) = login_camera_fov_for_world(&assets.world_name) {
            if let Projection::Perspective(perspective) = projection.as_mut() {
                perspective.fov = fov_radians;
                info!(
                    "Applied login camera FOV profile for {}: {:.1} degrees",
                    assets.world_name,
                    fov_radians.to_degrees()
                );
            }
        }

        commands.entity(camera_entity).insert((
            CameraTour {
                waypoints,
                current_index: 0,
                next_index: 1,
                progress: 0.0,
                speed: 1.0,
                active: true,
                loop_enabled: tour_data.r#loop,
            },
            CameraTourState { delay_timer: None },
        ));

        commands.spawn((CameraTourSetup, RuntimeSceneEntity));

        info!("Camera tour setup complete");
    } else {
        warn!("No 3D camera found for camera tour");
    }
}

/// System to update camera tour
pub fn update_camera_tour(
    debug_free_camera: Option<Res<DebugFreeCameraController>>,
    mut camera_query: Query<(&mut Transform, &mut CameraTour, &mut CameraTourState)>,
    time: Res<Time>,
) {
    if debug_free_camera
        .as_ref()
        .is_some_and(|controller| controller.enabled)
    {
        return;
    }

    let delta_seconds = time.delta_secs();

    for (mut transform, mut tour, mut state) in camera_query.iter_mut() {
        if !tour.active {
            continue;
        }

        if tour.waypoints.len() < 2 {
            continue;
        }

        // Check if we're in a delay
        if let Some(ref mut timer) = state.delay_timer {
            timer.tick(time.delta());
            if !timer.is_finished() {
                continue;
            } else {
                state.delay_timer = None;
            }
        }

        let waypoint_count = tour.waypoints.len();
        if tour.current_index >= waypoint_count {
            tour.current_index = 0;
        }
        if tour.next_index >= waypoint_count || tour.next_index == tour.current_index {
            tour.next_index = (tour.current_index + 1) % waypoint_count;
        }

        let mut current_wp = tour.waypoints[tour.current_index].clone();
        let mut next_wp = tour.waypoints[tour.next_index].clone();
        let segment_distance = current_wp.position.distance(next_wp.position);
        let movement_speed = current_wp.move_acceleration.max(0.1) * tour.speed.max(0.01);

        if segment_distance <= f32::EPSILON {
            if !advance_waypoint(&mut tour) {
                continue;
            }
            tour.progress = 0.0;
        } else {
            tour.progress += (movement_speed * delta_seconds) / segment_distance;
        }

        while tour.progress >= 1.0 && tour.active {
            tour.progress -= 1.0;
            let reached_waypoint = tour.next_index;

            if !advance_waypoint(&mut tour) {
                break;
            }

            let reached_delay = tour.waypoints[reached_waypoint].delay;
            if reached_delay > 0.0 {
                tour.progress = 0.0;
                state.delay_timer = Some(Timer::from_seconds(reached_delay, TimerMode::Once));
                break;
            }
        }

        if !tour.active {
            continue;
        }

        current_wp = tour.waypoints[tour.current_index].clone();
        next_wp = tour.waypoints[tour.next_index].clone();

        let interpolation_t = smoothstep(tour.progress.clamp(0.0, 1.0));
        let interpolated_position = current_wp.position.lerp(next_wp.position, interpolation_t);
        if !interpolated_position.is_finite() {
            warn!(
                "Camera tour produced non-finite position; disabling tour at segment {} -> {}",
                tour.current_index, tour.next_index
            );
            tour.active = false;
            continue;
        }

        let forward_direction =
            compute_forward_direction(&tour, tour.current_index, tour.next_index, interpolation_t);
        let look_distance = 500.0 + current_wp.distance_level.max(5.0) * 35.0;
        let look_height = current_wp
            .look_at
            .y
            .lerp(next_wp.look_at.y, interpolation_t);
        let mut desired_look_at = interpolated_position + forward_direction * look_distance;
        desired_look_at.y = look_height;

        let mut view_direction = desired_look_at - interpolated_position;
        if view_direction.length_squared() <= f32::EPSILON || !view_direction.is_finite() {
            view_direction = Vec3::new(forward_direction.x, -0.2, forward_direction.z);
        }
        if view_direction.y > -0.05 {
            view_direction.y = -0.05;
        }

        transform.translation = interpolated_position;
        let normalized_view_direction = view_direction.normalize_or_zero();
        if normalized_view_direction.length_squared() > f32::EPSILON {
            let desired_rotation = Transform::from_translation(Vec3::ZERO)
                .looking_to(normalized_view_direction, Vec3::Y)
                .rotation;
            if desired_rotation.is_finite() {
                let rotate_factor = (delta_seconds * 4.0).clamp(0.0, 1.0);
                transform.rotation = transform.rotation.slerp(desired_rotation, rotate_factor);
            }
        }
    }
}

fn advance_waypoint(tour: &mut CameraTour) -> bool {
    tour.current_index = tour.next_index;
    if tour.current_index + 1 >= tour.waypoints.len() {
        if tour.loop_enabled {
            tour.next_index = 0;
            true
        } else {
            tour.active = false;
            false
        }
    } else {
        tour.next_index = tour.current_index + 1;
        true
    }
}

/// Smoothstep interpolation function
fn smoothstep(t: f32) -> f32 {
    let x = t.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

fn compute_forward_direction(
    tour: &CameraTour,
    current_index: usize,
    next_index: usize,
    progress: f32,
) -> Vec3 {
    let waypoint_count = tour.waypoints.len();
    if waypoint_count < 2 {
        return Vec3::Z;
    }

    let prev_index = if current_index > 0 {
        current_index - 1
    } else {
        waypoint_count - 1
    };
    let next_next_index = (next_index + 1) % waypoint_count;

    let base_direction = planar_direction(
        tour.waypoints[current_index].position,
        tour.waypoints[next_index].position,
    )
    .unwrap_or(Vec3::Z);
    let previous_direction = planar_direction(
        tour.waypoints[prev_index].position,
        tour.waypoints[current_index].position,
    )
    .unwrap_or(base_direction);
    let next_direction = planar_direction(
        tour.waypoints[next_index].position,
        tour.waypoints[next_next_index].position,
    )
    .unwrap_or(base_direction);

    let forward = if progress < 0.35 {
        let blend = (progress / 0.35).clamp(0.0, 1.0);
        previous_direction.lerp(base_direction, blend)
    } else if progress > 0.65 {
        let blend = ((progress - 0.65) / 0.35).clamp(0.0, 1.0);
        base_direction.lerp(next_direction, blend)
    } else {
        base_direction
    };

    let normalized = Vec3::new(forward.x, 0.0, forward.z).normalize_or_zero();
    if normalized.length_squared() > f32::EPSILON {
        normalized
    } else {
        base_direction
    }
}

fn planar_direction(from: Vec3, to: Vec3) -> Option<Vec3> {
    let mut direction = to - from;
    direction.y = 0.0;
    let normalized = direction.normalize_or_zero();
    if normalized.length_squared() > f32::EPSILON {
        Some(normalized)
    } else {
        None
    }
}
