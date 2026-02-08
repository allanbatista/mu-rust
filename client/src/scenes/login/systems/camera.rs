use crate::scenes::login::LoginSceneAssets;
use crate::scenes::login::components::*;
use bevy::prelude::*;

/// Marker for camera tour setup
#[derive(Component)]
pub struct CameraTourSetup;

/// System to setup camera tour once assets are loaded
pub fn setup_camera_tour(
    mut commands: Commands,
    assets: Res<LoginSceneAssets>,
    camera_tour_data: Res<Assets<CameraTourData>>,
    mut camera_query: Query<Entity, (With<Camera3d>, Without<CameraTour>)>,
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

    let Some(tour_data) = camera_tour_data.get(&world.camera_tour) else {
        return;
    };

    info!(
        "Setting up camera tour with {} waypoints",
        tour_data.waypoints.len()
    );

    // Convert waypoint data to component format
    let waypoints: Vec<CameraWaypoint> = tour_data
        .waypoints
        .iter()
        .map(|wp| CameraWaypoint {
            index: wp.index,
            position: Vec3::from(wp.position),
            look_at: Vec3::from(wp.look_at),
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
    if let Ok(camera_entity) = camera_query.get_single_mut() {
        commands.entity(camera_entity).insert((
            CameraTour {
                waypoints,
                current_index: 0,
                next_index: 1,
                progress: 0.0,
                speed: 1.0,
                active: true,
                loop_enabled: tour_data.r#loop,
                blend_distance: tour_data.blend_distance,
            },
            CameraTourState { delay_timer: None },
        ));

        commands.spawn((CameraTourSetup, LoginSceneEntity));

        info!("Camera tour setup complete");
    } else {
        warn!("No 3D camera found for camera tour");
    }
}

/// System to update camera tour
pub fn update_camera_tour(
    mut camera_query: Query<(&mut Transform, &mut CameraTour, &mut CameraTourState)>,
    time: Res<Time>,
) {
    let delta_seconds = time.delta_seconds();

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
            if !timer.finished() {
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
