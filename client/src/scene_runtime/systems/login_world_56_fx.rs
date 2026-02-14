use super::{SceneObjectsSpawned, SkyboxSpawned};
use crate::bevy_compat::*;
use crate::infra::assets::resolve_asset_path;
use crate::scene_runtime::components::*;
use crate::scene_runtime::state::RuntimeSceneAssets;
use bevy::math::primitives::Cuboid;
use bevy::prelude::*;
use rand::Rng;
use std::f32::consts::TAU;

const WORLD_56_NAME: &str = "world_56";
const DARK_LORD_YAW_DEGREES: f32 = -32.0;
const DARK_LORD_SCALE: f32 = 1.2;

#[derive(Component)]
pub struct World56LoginFxInitialized;

#[derive(Component)]
pub struct World56SkyVortex {
    angular_speed_radians: f32,
    pulse_speed: f32,
    pulse_phase: f32,
    base_height: f32,
}

#[derive(Component)]
pub struct World56FlyingMonster {
    origin: Vec3,
    base_rotation: Quat,
    radius: f32,
    vertical_amplitude: f32,
    angular_speed: f32,
    phase: f32,
}

#[derive(Component)]
pub struct World56MeteorSpawner {
    base_origin: Vec3,
    timer: Timer,
}

#[derive(Component)]
pub struct World56Meteor {
    velocity: Vec3,
    lifetime: f32,
    spin_speed: f32,
}

#[derive(Component)]
pub struct World56DarkLordIdle {
    base_position: Vec3,
    phase: f32,
}

#[derive(Default)]
pub(crate) struct World56MeteorAssets {
    mesh: Option<Handle<Mesh>>,
    material: Option<Handle<StandardMaterial>>,
}

/// Initialize missing legacy effects for WD_55 login scene (world_56 assets).
pub fn initialize_world_56_login_fx(
    mut commands: Commands,
    assets: Res<RuntimeSceneAssets>,
    asset_server: Res<AssetServer>,
    scene_objects_spawned: Query<(), With<SceneObjectsSpawned>>,
    initialized: Query<(), With<World56LoginFxInitialized>>,
    mut scene_objects: Query<
        (
            Entity,
            &SceneObjectKind,
            &Transform,
            Option<&ParticleEmitter>,
        ),
        With<SceneObject>,
    >,
) {
    if !assets.loaded
        || !is_world_56(&assets)
        || scene_objects_spawned.is_empty()
        || !initialized.is_empty()
    {
        return;
    }

    let mut has_meteor_spawner = false;
    let mut moving_monster_index = 0u32;

    for (entity, kind, transform, emitter) in &mut scene_objects {
        match kind.0 {
            // WD_55 sky rings (C++: types 86 and 90)
            86 => {
                commands.entity(entity).insert(World56SkyVortex {
                    angular_speed_radians: 36.0_f32.to_radians(),
                    pulse_speed: 0.65,
                    pulse_phase: phase_from_entity(entity),
                    base_height: transform.translation.y,
                });
            }
            90 => {
                commands.entity(entity).insert(World56SkyVortex {
                    angular_speed_radians: 13.8_f32.to_radians(),
                    pulse_speed: 0.4,
                    pulse_phase: phase_from_entity(entity),
                    base_height: transform.translation.y,
                });
            }
            // Animated flying creatures around login map center.
            83 | 88 => {
                let phase = moving_monster_index as f32 * 0.61;
                let radius = 26.0 + (moving_monster_index % 6) as f32 * 8.5;
                let vertical = 10.0 + (moving_monster_index % 5) as f32 * 3.5;
                let speed = 0.18 + (moving_monster_index % 4) as f32 * 0.045;
                moving_monster_index = moving_monster_index.saturating_add(1);

                commands.entity(entity).insert(World56FlyingMonster {
                    origin: transform.translation,
                    base_rotation: transform.rotation,
                    radius,
                    vertical_amplitude: vertical,
                    angular_speed: speed,
                    phase,
                });
            }
            // C++ hides object mesh and renders cloud particles instead (type 84).
            84 => {
                let mut entity_commands = commands.entity(entity);
                entity_commands.insert(Visibility::Hidden);
                if emitter.is_none() {
                    entity_commands.insert(world_56_cloud_emitter());
                }
            }
            // Meteor source object (C++: type 89). Hide mesh and keep as emitter anchor.
            89 => {
                has_meteor_spawner = true;
                commands.entity(entity).insert((
                    Visibility::Hidden,
                    World56MeteorSpawner {
                        base_origin: transform.translation,
                        timer: Timer::from_seconds(8.0, TimerMode::Repeating),
                    },
                ));
            }
            _ => {}
        }
    }

    if !has_meteor_spawner {
        // Fallback to the original meteor source position from converted world data.
        commands.spawn((
            RuntimeSceneEntity,
            World56MeteorSpawner {
                base_origin: Vec3::new(8_954.0, 492.0, 24_177.0),
                timer: Timer::from_seconds(8.0, TimerMode::Repeating),
            },
            SpatialBundle::default(),
        ));
    }

    spawn_world_56_dark_lord(&mut commands, &asset_server);
    commands.spawn((
        RuntimeSceneEntity,
        World56LoginFxInitialized,
        SpatialBundle::default(),
    ));
}

/// Red tint + subtle motion for login skybox.
pub fn animate_world_56_skybox(
    assets: Res<RuntimeSceneAssets>,
    time: Res<Time>,
    mut skyboxes: Query<(&MeshMaterial3d<StandardMaterial>, &mut Transform), With<SkyboxSpawned>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if !is_world_56(&assets) {
        return;
    }

    let elapsed = time.elapsed_secs();
    for (material_handle, mut transform) in &mut skyboxes {
        transform.rotate_local_y(0.02 * time.delta_secs());

        if let Some(material) = materials.get_mut(&material_handle.0) {
            let pulse = (elapsed * 0.22).sin() * 0.5 + 0.5;
            let red = 0.76 + pulse * 0.2;
            let green = 0.08 + pulse * 0.09;
            let blue = 0.07 + pulse * 0.06;
            material.base_color = Color::srgb(red, green, blue);
            material.unlit = true;
        }
    }
}

/// Rotate WD_55 sky objects (types 86/90) like the original client.
pub fn animate_world_56_sky_vortex_objects(
    assets: Res<RuntimeSceneAssets>,
    time: Res<Time>,
    mut query: Query<(&SceneObjectKind, &mut Transform, &World56SkyVortex), With<SceneObject>>,
) {
    if !is_world_56(&assets) {
        return;
    }

    let elapsed = time.elapsed_secs();
    for (kind, mut transform, vortex) in &mut query {
        transform.rotate_local_z(-vortex.angular_speed_radians * time.delta_secs());

        if kind.0 == 86 {
            let wobble = (elapsed * vortex.pulse_speed + vortex.pulse_phase).sin();
            transform.translation.y = vortex.base_height + wobble * 4.0;
        }
    }
}

/// Move selected scene objects to emulate flying monsters in the login map.
pub fn animate_world_56_flying_monsters(
    assets: Res<RuntimeSceneAssets>,
    time: Res<Time>,
    mut monsters: Query<(&mut Transform, &mut World56FlyingMonster), With<SceneObject>>,
) {
    if !is_world_56(&assets) {
        return;
    }

    let elapsed = time.elapsed_secs();
    let delta = time.delta_secs();
    for (mut transform, mut monster) in &mut monsters {
        monster.phase = (monster.phase + monster.angular_speed * delta).rem_euclid(TAU);

        let orbit = Vec3::new(
            monster.phase.cos() * monster.radius,
            (elapsed * 0.9 + monster.phase).sin() * monster.vertical_amplitude,
            monster.phase.sin() * monster.radius,
        );

        transform.translation = monster.origin + orbit;
        transform.rotation = monster.base_rotation * Quat::from_rotation_y(monster.phase + 0.4);
    }
}

/// Spawn meteor entities from world_56 spawners.
pub fn spawn_world_56_meteors(
    mut commands: Commands,
    assets: Res<RuntimeSceneAssets>,
    time: Res<Time>,
    mut spawners: Query<&mut World56MeteorSpawner>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meteor_assets: Local<World56MeteorAssets>,
) {
    if !is_world_56(&assets) {
        return;
    }

    let meteor_mesh = meteor_assets
        .mesh
        .get_or_insert_with(|| meshes.add(Mesh::from(Cuboid::new(160.0, 42.0, 42.0))))
        .clone();
    let meteor_material = meteor_assets
        .material
        .get_or_insert_with(|| {
            materials.add(StandardMaterial {
                base_color: Color::srgba(1.0, 0.32, 0.14, 0.96),
                alpha_mode: AlphaMode::Blend,
                unlit: true,
                cull_mode: None,
                ..default()
            })
        })
        .clone();

    let mut rng = rand::thread_rng();
    for mut spawner in &mut spawners {
        spawner.timer.tick(time.delta());
        let spawn_count = spawner.timer.times_finished_this_tick();
        if spawn_count == 0 {
            continue;
        }

        for _ in 0..spawn_count {
            let start = spawner.base_origin
                + Vec3::new(
                    rng.gen_range(-1024.0..=1024.0),
                    rng.gen_range(3000.0..=3600.0),
                    rng.gen_range(-1024.0..=1024.0),
                );
            let velocity = Vec3::new(
                rng.gen_range(-320.0..=-180.0),
                rng.gen_range(-640.0..=-520.0),
                rng.gen_range(-80.0..=80.0),
            );

            let mut transform = Transform::from_translation(start);
            transform.look_to(velocity.normalize_or_zero(), Vec3::Y);

            commands.spawn((
                RuntimeSceneEntity,
                World56Meteor {
                    velocity,
                    lifetime: rng.gen_range(5.0..=7.2),
                    spin_speed: rng.gen_range(-1.2..=1.2),
                },
                meteor_trail_emitter(),
                PbrBundle {
                    mesh: Mesh3d(meteor_mesh.clone()),
                    material: MeshMaterial3d(meteor_material.clone()),
                    transform,
                    ..default()
                },
            ));
        }
    }
}

/// Update meteor movement and lifetime.
pub fn update_world_56_meteors(
    mut commands: Commands,
    assets: Res<RuntimeSceneAssets>,
    time: Res<Time>,
    mut meteors: Query<(Entity, &mut Transform, &mut World56Meteor)>,
) {
    if !is_world_56(&assets) {
        return;
    }

    let dt = time.delta_secs();
    for (entity, mut transform, mut meteor) in &mut meteors {
        meteor.lifetime -= dt;
        meteor.velocity.y -= 70.0 * dt;
        transform.translation += meteor.velocity * dt;
        transform.rotate_local_z(meteor.spin_speed * dt);

        if meteor.velocity.length_squared() > 0.01 {
            transform.look_to(meteor.velocity.normalize(), Vec3::Y);
        }

        if meteor.lifetime <= 0.0 || transform.translation.y < -400.0 {
            commands.entity(entity).despawn();
        }
    }
}

/// Add a subtle idle animation to the Dark Lord spawn.
pub fn animate_world_56_dark_lord(
    assets: Res<RuntimeSceneAssets>,
    time: Res<Time>,
    mut dark_lord: Query<(&mut Transform, &World56DarkLordIdle)>,
) {
    if !is_world_56(&assets) {
        return;
    }

    let elapsed = time.elapsed_secs();
    for (mut transform, idle) in &mut dark_lord {
        transform.translation =
            idle.base_position + Vec3::Y * (elapsed * 0.7 + idle.phase).sin() * 8.0;
        transform.rotation =
            Quat::from_rotation_y(DARK_LORD_YAW_DEGREES.to_radians() + elapsed * 0.08);
    }
}

fn is_world_56(assets: &RuntimeSceneAssets) -> bool {
    if assets.world_name.eq_ignore_ascii_case(WORLD_56_NAME) {
        return true;
    }

    assets
        .world
        .as_ref()
        .map(|world| world.world_name.eq_ignore_ascii_case(WORLD_56_NAME))
        .unwrap_or(false)
}

fn spawn_world_56_dark_lord(commands: &mut Commands, asset_server: &AssetServer) {
    let base_position = Vec3::new(12_800.0, 170.0, 12_800.0);
    let root = commands
        .spawn((
            RuntimeSceneEntity,
            World56DarkLordIdle {
                base_position,
                phase: 0.35,
            },
            SpatialBundle {
                transform: Transform {
                    translation: base_position,
                    rotation: Quat::from_rotation_y(DARK_LORD_YAW_DEGREES.to_radians()),
                    scale: Vec3::splat(DARK_LORD_SCALE),
                },
                ..default()
            },
        ))
        .id();

    let dark_lord_parts = [
        "data/player/helm_class_05.glb#Scene0",
        "data/player/armor_class_05.glb#Scene0",
        "data/player/pant_class_05.glb#Scene0",
        "data/player/glove_class_05.glb#Scene0",
        "data/player/boot_class_05.glb#Scene0",
        "data/item/dark_lord_robe.glb#Scene0",
    ];

    commands.entity(root).with_children(|parent| {
        for scene_path in dark_lord_parts {
            let scene: Handle<Scene> = asset_server.load(resolve_asset_path(scene_path));
            parent.spawn(SceneBundle {
                scene: SceneRoot(scene),
                ..default()
            });
        }
    });
}

fn world_56_cloud_emitter() -> ParticleEmitter {
    ParticleEmitter {
        config: ParticleEmitterConfig {
            lifetime_range: (1.0, 2.2),
            initial_velocity: Vec3::new(0.0, 1.2, 0.0),
            velocity_variance: Vec3::new(0.9, 0.6, 0.9),
            scale_range: (45.0, 120.0),
            scale_variance: 18.0,
            color_start: Vec4::new(0.92, 0.92, 0.96, 0.45),
            color_end: Vec4::new(0.92, 0.92, 0.96, 0.0),
            texture_path: "data/effect/hart_particle02.png".to_string(),
            blend_mode: ParticleBlendMode::Alpha,
            rotation_speed: 0.35,
            max_particles: 96,
        },
        active: true,
        particles: Vec::new(),
        spawn_timer: Timer::from_seconds(0.12, TimerMode::Repeating),
    }
}

fn meteor_trail_emitter() -> ParticleEmitter {
    ParticleEmitter {
        config: ParticleEmitterConfig {
            lifetime_range: (0.25, 0.55),
            initial_velocity: Vec3::new(0.0, 30.0, 0.0),
            velocity_variance: Vec3::new(22.0, 14.0, 22.0),
            scale_range: (24.0, 58.0),
            scale_variance: 12.0,
            color_start: Vec4::new(1.0, 0.62, 0.2, 0.95),
            color_end: Vec4::new(0.95, 0.1, 0.02, 0.0),
            texture_path: "data/effect/flame_chrom2.png".to_string(),
            blend_mode: ParticleBlendMode::Additive,
            rotation_speed: 0.9,
            max_particles: 96,
        },
        active: true,
        particles: Vec::new(),
        spawn_timer: Timer::from_seconds(0.02, TimerMode::Repeating),
    }
}

fn phase_from_entity(entity: Entity) -> f32 {
    ((entity.index_u32() as f32 * 0.131) + (entity.generation().to_bits() as f32 * 0.071))
        .rem_euclid(TAU)
}
