use crate::scene_runtime::components::*;
use bevy::prelude::*;
use bevy::time::Timer;
use rand::Rng;
use std::f32::consts::TAU;

/// Spawn particle burst at the configured delay, then despawn after lifetime expires.
pub fn update_skill_impact_bursts(
    mut commands: Commands,
    time: Res<Time>,
    mut bursts: Query<(Entity, &mut SkillImpactBurst, &GlobalTransform)>,
) {
    let dt = time.delta_secs();

    for (entity, mut burst, transform) in &mut bursts {
        burst.elapsed += dt;

        if burst.elapsed >= burst.delay && !burst.fired {
            burst.fired = true;

            // Insert a particle emitter with pre-spawned particles for the burst
            let position = transform.translation();
            let mut particles = Vec::with_capacity(burst.burst_count as usize);
            let mut rng = rand::thread_rng();

            for _ in 0..burst.burst_count {
                let cfg = &burst.emitter_config;
                let lifetime =
                    rng.gen_range(cfg.lifetime_range.0..=cfg.lifetime_range.1);
                let velocity_offset = Vec3::new(
                    rng.gen_range(-cfg.velocity_variance.x..=cfg.velocity_variance.x),
                    rng.gen_range(-cfg.velocity_variance.y..=cfg.velocity_variance.y),
                    rng.gen_range(-cfg.velocity_variance.z..=cfg.velocity_variance.z),
                );
                let base_scale =
                    rng.gen_range(cfg.scale_range.0..=cfg.scale_range.1);
                let scale_jitter = if cfg.scale_variance > 0.0 {
                    rng.gen_range(-cfg.scale_variance..=cfg.scale_variance)
                } else {
                    0.0
                };
                let rotation = rng.gen_range(0.0..TAU);
                let rotation_speed_jitter = rng.gen_range(0.75..=1.25);

                particles.push(Particle {
                    position,
                    velocity: cfg.initial_velocity + velocity_offset,
                    lifetime: 0.0,
                    max_lifetime: lifetime,
                    scale: (base_scale + scale_jitter).max(0.01),
                    rotation,
                    rotation_speed: cfg.rotation_speed * rotation_speed_jitter,
                });
            }

            commands.entity(entity).insert(ParticleEmitter {
                config: burst.emitter_config.clone(),
                active: true,
                particles,
                // Set a very long interval so no new particles auto-spawn
                spawn_timer: Timer::from_seconds(999.0, TimerMode::Repeating),
            });
        }

        if burst.fired && burst.elapsed >= burst.delay + burst.lifetime_after_burst {
            commands.entity(entity).despawn();
        }
    }
}

/// Ramp a temporary light to peak intensity and then fade to zero over its lifetime.
pub fn update_skill_timed_lights(
    mut commands: Commands,
    time: Res<Time>,
    mut lights: Query<(Entity, &mut SkillTimedLight)>,
) {
    let dt = time.delta_secs();

    for (entity, mut light) in &mut lights {
        light.elapsed += dt;

        if light.elapsed >= light.lifetime {
            commands.entity(entity).remove::<DynamicLight>();
            commands.entity(entity).remove::<SkillTimedLight>();
            commands.entity(entity).despawn();
            continue;
        }

        // Compute intensity: ramp up to peak at peak_time, then fade to 0 by lifetime
        let intensity = if light.elapsed < light.peak_time {
            // Ramp up phase
            let t = if light.peak_time > 0.0 {
                light.elapsed / light.peak_time
            } else {
                1.0
            };
            light.base_intensity + (light.peak_intensity - light.base_intensity) * t
        } else {
            // Fade down phase
            let remaining = light.lifetime - light.peak_time;
            let t = if remaining > 0.0 {
                1.0 - (light.elapsed - light.peak_time) / remaining
            } else {
                0.0
            };
            light.peak_intensity * t.clamp(0.0, 1.0)
        };

        commands.entity(entity).insert(DynamicLight {
            color: light.color,
            intensity,
            range: light.range,
            flicker: None,
        });
    }
}
