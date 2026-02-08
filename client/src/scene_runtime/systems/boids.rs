use crate::scene_runtime::components::*;
use bevy::prelude::*;

/// System to update boid movement and animation
pub fn update_boids(
    mut boids: Query<(&mut Transform, &mut Boid, &mut BoidFlightPattern)>,
    time: Res<Time>,
) {
    for (mut transform, mut boid, mut pattern) in boids.iter_mut() {
        pattern.time += time.delta_seconds();

        match &pattern.pattern_type {
            FlightPattern::Circular { radius, speed } => {
                let angle = pattern.time * speed;

                // Calculate circular position with height variation
                let offset = Vec3::new(
                    angle.cos() * radius,
                    (pattern.time * 0.5).sin() * 5.0, // Wing flapping height variation
                    angle.sin() * radius,
                );

                transform.translation = boid.spawn_point + offset;

                // Orient bird in direction of movement
                let forward = Vec3::new(-angle.sin(), 0.0, angle.cos()).normalize();
                if forward.length_squared() > 0.0 {
                    transform.look_to(forward, Vec3::Y);
                }
            }
            FlightPattern::Patrol { points, current } => {
                // Patrol between waypoints
                if points.is_empty() {
                    continue;
                }

                let target = points[*current];
                let direction = (target - transform.translation).normalize();

                // Move towards target
                transform.translation += direction * boid.velocity.length() * time.delta_seconds();

                // Check if reached target
                if transform.translation.distance(target) < 1.0 {
                    // Move to next point
                    let mut new_current = *current + 1;
                    if new_current >= points.len() {
                        new_current = 0;
                    }
                    if let FlightPattern::Patrol { current, .. } = &mut pattern.pattern_type {
                        *current = new_current;
                    }
                }

                // Orient towards movement direction
                if direction.length_squared() > 0.0 {
                    transform.look_to(direction, Vec3::Y);
                }
            }
        }

        // Update animation timer
        boid.animation_timer.tick(time.delta());
        // In a full implementation, this would trigger animation frame changes
    }
}
