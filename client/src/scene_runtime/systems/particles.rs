use crate::scene_runtime::components::*;
use bevy::prelude::*;
use rand::Rng;

/// System to update particle emitters
pub fn update_particle_emitters(
    mut emitters: Query<(&mut ParticleEmitter, &GlobalTransform)>,
    time: Res<Time>,
) {
    for (mut emitter, transform) in emitters.iter_mut() {
        if !emitter.active {
            continue;
        }

        // Update spawn timer
        emitter.spawn_timer.tick(time.delta());

        // Spawn new particles
        while emitter.spawn_timer.just_finished() {
            spawn_particle(&mut emitter, transform.translation());
        }

        // Update existing particles
        update_particles(&mut emitter.particles, time.delta_seconds());
    }
}

/// Spawn a new particle
fn spawn_particle(emitter: &mut ParticleEmitter, position: Vec3) {
    let mut rng = rand::thread_rng();

    let lifetime = rng.gen_range(emitter.config.lifetime_range.0..=emitter.config.lifetime_range.1);

    let velocity_offset = Vec3::new(
        rng.gen_range(-emitter.config.velocity_variance.x..=emitter.config.velocity_variance.x),
        rng.gen_range(-emitter.config.velocity_variance.y..=emitter.config.velocity_variance.y),
        rng.gen_range(-emitter.config.velocity_variance.z..=emitter.config.velocity_variance.z),
    );

    let velocity = emitter.config.initial_velocity + velocity_offset;

    let scale_variance =
        rng.gen_range(-emitter.config.scale_variance..=emitter.config.scale_variance);
    let scale =
        rng.gen_range(emitter.config.scale_range.0..=emitter.config.scale_range.1) + scale_variance;

    let particle = Particle {
        position,
        velocity,
        lifetime: 0.0,
        max_lifetime: lifetime,
        scale,
        rotation: 0.0,
    };

    emitter.particles.push(particle);
}

/// Update all particles in the system
fn update_particles(particles: &mut Vec<Particle>, delta: f32) {
    // Update and remove dead particles
    particles.retain_mut(|particle| {
        particle.lifetime += delta;

        if particle.lifetime >= particle.max_lifetime {
            return false; // Remove dead particle
        }

        // Update physics
        particle.position += particle.velocity * delta;

        // Apply gravity (simple simulation)
        particle.velocity.y -= 9.8 * delta;

        true
    });
}

// Note: Actual rendering of particles will be handled by a separate rendering system
// or integrated with bevy_hanabi in the future
