use crate::scene_runtime::components::*;
use bevy::prelude::*;

/// System to update boid movement and animation
pub fn update_boids(
    mut boids: Query<(&mut Transform, &mut Boid, &mut BoidFlightPattern)>,
    time: Res<Time>,
) {
    for (mut transform, mut boid, mut pattern) in boids.iter_mut() {
        pattern.time += time.delta_secs();

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
        }

        // Update animation timer
        boid.animation_timer.tick(time.delta());
        // In a full implementation, this would trigger animation frame changes
    }
}
