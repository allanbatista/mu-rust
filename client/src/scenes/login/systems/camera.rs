use crate::scenes::login::components::*;
use crate::scenes::login::LoginSceneAssets;
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

    let Some(tour_data) = camera_tour_data.get(&assets.camera_tour) else {
        return;
    };

    info!("Setting up camera tour with {} waypoints", tour_data.waypoints.len());

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
            CameraTourState {
                delay_timer: None,
            },
        ));

        commands.spawn(CameraTourSetup);

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
    for (mut transform, mut tour, mut state) in camera_query.iter_mut() {
        if !tour.active {
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

        if tour.waypoints.is_empty() {
            continue;
        }

        // Clone waypoints to avoid borrow issues
        let current_wp = tour.waypoints[tour.current_index].clone();
        let next_wp = tour.waypoints[tour.next_index].clone();

        // Calculate distance to next waypoint
        let distance_to_next = transform.translation.distance(next_wp.position);

        // Determine if we should use smooth blending
        let should_blend = distance_to_next < tour.blend_distance;

        // Update progress
        let speed = current_wp.move_acceleration * time.delta_seconds();
        let total_distance = current_wp.position.distance(next_wp.position);
        if total_distance > 0.0 {
            tour.progress += speed / total_distance;
        }

        if tour.progress >= 1.0 {
            // Reached waypoint
            tour.current_index = tour.next_index;
            tour.next_index = if tour.current_index + 1 >= tour.waypoints.len() {
                if tour.loop_enabled {
                    0
                } else {
                    tour.active = false;
                    continue;
                }
            } else {
                tour.current_index + 1
            };
            tour.progress = 0.0;

            // Apply delay if specified
            if current_wp.delay > 0.0 {
                state.delay_timer = Some(Timer::from_seconds(
                    current_wp.delay,
                    TimerMode::Once,
                ));
            }
        }

        // Interpolate position
        let interpolated_pos = if should_blend {
            // Smooth blending near waypoint
            let blend_factor = 1.0 - (distance_to_next / tour.blend_distance);
            let blend_t = smoothstep(tour.progress, blend_factor);
            current_wp.position.lerp(next_wp.position, blend_t)
        } else {
            current_wp.position.lerp(next_wp.position, tour.progress)
        };

        // Interpolate look_at
        let interpolated_look_at = current_wp.look_at.lerp(next_wp.look_at, tour.progress);

        // Update transform
        transform.translation = interpolated_pos;
        transform.look_at(interpolated_look_at, Vec3::Y);
    }
}

/// Smoothstep interpolation function
fn smoothstep(t: f32, blend: f32) -> f32 {
    let x = (t * blend).clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}
