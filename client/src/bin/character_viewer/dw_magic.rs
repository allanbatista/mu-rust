use super::*;
use rand::Rng;
use std::f32::consts::TAU;

const DW_BASE_FPS: f32 = 25.0;
const DW_METEORITE_FALL_RATIO: f32 = 0.55;
const EVIL_SPIRIT_BOLT_COUNT: usize = 8;
const EVIL_SPIRIT_LIGHTNING_TAILS: usize = 6;
const EVIL_SPIRIT_INITIAL_HEIGHT: f32 = 100.0;
const EVIL_SPIRIT_TARGET_HEIGHT: f32 = 80.0;
const EVIL_SPIRIT_LIFETIME_FRAMES: f32 = 49.0;
const EVIL_SPIRIT_LIFETIME_SECONDS: f32 = EVIL_SPIRIT_LIFETIME_FRAMES / DW_BASE_FPS;
const EVIL_SPIRIT_MAIN_SCALE: f32 = 1.05;
const EVIL_SPIRIT_VISUAL_SCALE: f32 = 0.42;
const EVIL_SPIRIT_TURN_RATE: f32 = 10.0;
const EVIL_SPIRIT_VELOCITY: f32 = 40.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DwMagicKind {
    Meteorite,
    Lightning,
    FireBall,
    Flame,
    Twister,
    EvilSpirit,
    HellFire,
    AquaBeam,
    Inferno,
    EnergyBall,
    IceStorm,
}

#[derive(Component)]
pub(super) struct DwMagicRuntime {
    kind: DwMagicKind,
    caster_entity: Entity,
    source_pos: Vec3,
    target_pos: Vec3,
    elapsed_secs: f32,
    total_duration_secs: f32,
    travel_duration_secs: f32,
    emission_accumulator_secs: f32,
    auxiliary_accumulator_frames: f32,
    impact_triggered: bool,
    initialized: bool,
    evil_spirit_states: [EvilSpiritState; EVIL_SPIRIT_BOLT_COUNT],
    evil_spirit_initialized: bool,
    random_phase: f32,
}

#[derive(Clone, Copy)]
struct EvilSpiritState {
    position: Vec3,
    tails: [Vec3; EVIL_SPIRIT_LIGHTNING_TAILS + 1],
    num_tails: u8,
    life_time: f32,
    angle_x: f32,
    angle_z: f32,
    direction_x: f32,
    direction_z: f32,
    scale: f32,
    active: bool,
}

impl Default for EvilSpiritState {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            tails: [Vec3::ZERO; EVIL_SPIRIT_LIGHTNING_TAILS + 1],
            num_tails: 0,
            life_time: EVIL_SPIRIT_LIFETIME_SECONDS,
            angle_x: 0.0,
            angle_z: 0.0,
            direction_x: 0.0,
            direction_z: 0.0,
            scale: EVIL_SPIRIT_MAIN_SCALE,
            active: false,
        }
    }
}

pub(super) fn spawn_dw_magic_skill_vfx(
    commands: &mut Commands,
    asset_server: &AssetServer,
    caster_entity: Entity,
    caster_pos: Vec3,
    target_pos: Vec3,
    skill_duration: f32,
    vfx: SkillVfxProfile,
) -> bool {
    let kind = match vfx {
        SkillVfxProfile::Meteorite => DwMagicKind::Meteorite,
        SkillVfxProfile::Lightning => DwMagicKind::Lightning,
        SkillVfxProfile::FireBall => DwMagicKind::FireBall,
        SkillVfxProfile::Flame => DwMagicKind::Flame,
        SkillVfxProfile::Twister => DwMagicKind::Twister,
        SkillVfxProfile::EvilSpirit => DwMagicKind::EvilSpirit,
        SkillVfxProfile::HellFire => DwMagicKind::HellFire,
        SkillVfxProfile::AquaBeam => DwMagicKind::AquaBeam,
        SkillVfxProfile::Inferno => DwMagicKind::Inferno,
        SkillVfxProfile::EnergyBall => DwMagicKind::EnergyBall,
        SkillVfxProfile::IceStorm => DwMagicKind::IceStorm,
        _ => return false,
    };

    let distance = caster_pos.distance(target_pos);
    let (total_duration_secs, travel_duration_secs) =
        timing_for_kind(kind, distance, skill_duration);
    let source_pos = source_position_for_kind(kind, caster_pos, target_pos);

    let random_phase = ((caster_pos.x + target_pos.z) * 0.013).sin().abs();
    let effect_entity = commands
        .spawn((
            SkillVfx,
            DwMagicRuntime {
                kind,
                caster_entity,
                source_pos,
                target_pos,
                elapsed_secs: 0.0,
                total_duration_secs,
                travel_duration_secs,
                emission_accumulator_secs: 0.0,
                auxiliary_accumulator_frames: 0.0,
                impact_triggered: false,
                initialized: false,
                evil_spirit_states: [EvilSpiritState::default(); EVIL_SPIRIT_BOLT_COUNT],
                evil_spirit_initialized: false,
                random_phase,
            },
            Transform::from_translation(source_pos),
            GlobalTransform::default(),
        ))
        .id();

    initialize_dw_effect(
        commands,
        asset_server,
        effect_entity,
        kind,
        caster_entity,
        source_pos,
        target_pos,
        total_duration_secs,
        travel_duration_secs,
    );

    true
}

pub(super) fn update_dw_magic_effects(
    mut commands: Commands,
    time: Res<Time>,
    asset_server: Res<AssetServer>,
    caster_transforms: Query<&GlobalTransform>,
    mut effects: Query<(Entity, &mut DwMagicRuntime, &mut Transform), With<SkillVfx>>,
) {
    let dt = time.delta_secs();
    let frame_factor = dt * DW_BASE_FPS;
    let mut rng = rand::thread_rng();

    for (effect_entity, mut effect, mut effect_transform) in &mut effects {
        if !effect.initialized {
            effect.initialized = true;
        }

        effect.elapsed_secs += dt;
        effect.emission_accumulator_secs += dt;
        effect.auxiliary_accumulator_frames += frame_factor;

        match effect.kind {
            DwMagicKind::Meteorite => update_meteorite_effect(
                &mut commands,
                &asset_server,
                &mut effect,
                &mut effect_transform,
                &mut rng,
            ),
            DwMagicKind::Lightning => {
                update_lightning_effect(&mut commands, &asset_server, &mut effect, &mut rng)
            }
            DwMagicKind::FireBall => update_fire_ball_effect(
                &mut commands,
                &asset_server,
                &mut effect,
                &mut effect_transform,
                &mut rng,
            ),
            DwMagicKind::Flame => {
                update_flame_effect(&mut commands, &asset_server, &mut effect, &mut rng)
            }
            DwMagicKind::Twister => {
                update_twister_effect(&mut commands, &asset_server, &mut effect, &mut rng)
            }
            DwMagicKind::EvilSpirit => update_evil_spirit_effect(
                &mut commands,
                &asset_server,
                &caster_transforms,
                &mut effect,
                frame_factor,
                &mut rng,
            ),
            DwMagicKind::HellFire => update_hell_fire_effect(
                &mut commands,
                &asset_server,
                &caster_transforms,
                &mut effect,
                &mut effect_transform,
                &mut rng,
            ),
            DwMagicKind::AquaBeam => {
                update_aqua_beam_effect(&mut commands, &asset_server, &mut effect, &mut rng)
            }
            DwMagicKind::Inferno => {
                update_inferno_effect(&mut commands, &asset_server, &mut effect, &mut rng)
            }
            DwMagicKind::EnergyBall => update_energy_ball_effect(
                &mut commands,
                &asset_server,
                &mut effect,
                &mut effect_transform,
                &mut rng,
            ),
            DwMagicKind::IceStorm => {
                update_ice_storm_effect(&mut commands, &asset_server, &mut effect, &mut rng)
            }
        }

        if effect.elapsed_secs >= effect.total_duration_secs {
            commands.entity(effect_entity).despawn();
        }
    }
}

pub(super) fn preload_dw_magic_skill_vfx_assets(
    skills: &[SkillEntry],
    asset_server: &AssetServer,
    preload_cache: &mut SkillVfxPreloadCache,
) {
    if !skills.iter().any(|skill| is_dw_magic_profile(skill.vfx)) {
        return;
    }

    let _: Handle<Image> = asset_server.load(resolve_asset_path("data/effect/spark_01.png"));
    let _: Handle<Image> = asset_server.load(resolve_asset_path("data/effect/spark_02.png"));
    let _: Handle<Image> = asset_server.load(resolve_asset_path("data/effect/spark_03.png"));
    let _: Handle<Image> = asset_server.load(resolve_asset_path("data/effect/fire_01.png"));
    let _: Handle<Image> = asset_server.load(resolve_asset_path("data/effect/smoke_01.png"));
    let _: Handle<Image> = asset_server.load(resolve_asset_path("data/effect/joint_spirit_01.png"));

    for skill in skills {
        match skill.vfx {
            SkillVfxProfile::Meteorite => {
                preload_skill_vfx_asset("data/skill/storm_01.glb", asset_server, preload_cache);
                preload_skill_vfx_asset("data/skill/blast_01.glb", asset_server, preload_cache);
                preload_skill_vfx_asset("data/skill/fire_01.glb", asset_server, preload_cache);
            }
            SkillVfxProfile::Lightning => {
                preload_skill_vfx_asset("data/skill/laser_01.glb", asset_server, preload_cache);
            }
            SkillVfxProfile::FireBall => {
                preload_skill_vfx_asset("data/skill/fire_01.glb", asset_server, preload_cache);
                preload_skill_vfx_asset("data/skill/magic_01.glb", asset_server, preload_cache);
                preload_skill_vfx_asset("data/skill/blast_01.glb", asset_server, preload_cache);
            }
            SkillVfxProfile::Flame => {
                preload_skill_vfx_asset("data/skill/fire_01.glb", asset_server, preload_cache);
                preload_skill_vfx_asset("data/skill/storm_01.glb", asset_server, preload_cache);
            }
            SkillVfxProfile::Twister => {
                preload_skill_vfx_asset("data/skill/storm_01.glb", asset_server, preload_cache);
                preload_skill_vfx_asset("data/skill/laser_01.glb", asset_server, preload_cache);
                preload_skill_vfx_asset("data/skill/ground_stone.glb", asset_server, preload_cache);
            }
            SkillVfxProfile::EvilSpirit => {
                preload_skill_vfx_asset(
                    "data/skill/evil_spirit_2.glb",
                    asset_server,
                    preload_cache,
                );
                preload_skill_vfx_asset("data/skill/storm_01.glb", asset_server, preload_cache);
            }
            SkillVfxProfile::HellFire => {
                preload_skill_vfx_asset("data/skill/circle_01.glb", asset_server, preload_cache);
                preload_skill_vfx_asset("data/skill/circle_02.glb", asset_server, preload_cache);
                preload_skill_vfx_asset("data/skill/fire_01.glb", asset_server, preload_cache);
                preload_skill_vfx_asset("data/skill/ground_stone.glb", asset_server, preload_cache);
            }
            SkillVfxProfile::AquaBeam => {
                preload_skill_vfx_asset("data/skill/ice_01.glb", asset_server, preload_cache);
                preload_skill_vfx_asset("data/skill/storm_01.glb", asset_server, preload_cache);
            }
            SkillVfxProfile::Inferno => {
                preload_skill_vfx_asset("data/skill/inferno_01.glb", asset_server, preload_cache);
                preload_skill_vfx_asset("data/skill/fire_01.glb", asset_server, preload_cache);
                preload_skill_vfx_asset("data/skill/blast_01.glb", asset_server, preload_cache);
                preload_skill_vfx_asset("data/skill/ground_stone.glb", asset_server, preload_cache);
            }
            SkillVfxProfile::EnergyBall => {
                preload_skill_vfx_asset("data/skill/air_force.glb", asset_server, preload_cache);
                preload_skill_vfx_asset("data/skill/storm_01.glb", asset_server, preload_cache);
            }
            SkillVfxProfile::IceStorm => {
                preload_skill_vfx_asset("data/skill/blizzard.glb", asset_server, preload_cache);
                preload_skill_vfx_asset("data/skill/ice_01.glb", asset_server, preload_cache);
            }
            _ => {}
        }
    }
}

fn timing_for_kind(kind: DwMagicKind, distance: f32, skill_duration: f32) -> (f32, f32) {
    match kind {
        DwMagicKind::Meteorite => {
            let total = (distance / 900.0).clamp(0.8, 2.5).max(1.0);
            (total, total * DW_METEORITE_FALL_RATIO)
        }
        DwMagicKind::Lightning => {
            let total = (distance / 700.0).clamp(0.10, 3.0).max(0.20);
            (total, total)
        }
        DwMagicKind::FireBall => {
            let travel = (distance / 1800.0).clamp(0.18, 1.20);
            (travel + 0.22, travel)
        }
        DwMagicKind::Flame => {
            let total = if distance > 0.0 { 2.2 } else { 1.4 };
            (total, total)
        }
        DwMagicKind::Twister => {
            let total = 59.0 / DW_BASE_FPS;
            (total, total)
        }
        DwMagicKind::EvilSpirit => {
            let total = 49.0 / DW_BASE_FPS;
            (total, total)
        }
        DwMagicKind::HellFire => {
            let total = (45.0 / DW_BASE_FPS).max(skill_duration * 0.70);
            (total, total)
        }
        DwMagicKind::AquaBeam => {
            let total = 20.0 / DW_BASE_FPS;
            (total, total)
        }
        DwMagicKind::Inferno => {
            let total = (20.0 / DW_BASE_FPS).max(skill_duration * 0.40);
            (total, total)
        }
        DwMagicKind::EnergyBall => {
            let travel = (distance / 2400.0).clamp(0.16, 1.10);
            (travel + 0.18, travel)
        }
        DwMagicKind::IceStorm => {
            let total = 120.0 / DW_BASE_FPS;
            (total, total)
        }
    }
}

fn source_position_for_kind(kind: DwMagicKind, caster_pos: Vec3, target_pos: Vec3) -> Vec3 {
    match kind {
        DwMagicKind::Meteorite => target_pos + Vec3::new(0.0, 360.0, -90.0),
        DwMagicKind::FireBall
        | DwMagicKind::EnergyBall
        | DwMagicKind::Lightning
        | DwMagicKind::AquaBeam => caster_pos + Vec3::new(0.0, 110.0, 0.0),
        DwMagicKind::HellFire => caster_pos,
        _ => target_pos,
    }
}

#[allow(clippy::too_many_arguments)]
fn initialize_dw_effect(
    commands: &mut Commands,
    asset_server: &AssetServer,
    effect_entity: Entity,
    kind: DwMagicKind,
    caster_entity: Entity,
    source_pos: Vec3,
    target_pos: Vec3,
    total_duration_secs: f32,
    travel_duration_secs: f32,
) {
    match kind {
        DwMagicKind::Meteorite => {
            spawn_skill_vfx_scene(
                commands,
                asset_server,
                "data/skill/fire_01.glb",
                source_pos,
                1.25,
                travel_duration_secs + 0.12,
                Some((effect_entity, Vec3::ZERO)),
            );
            spawn_skill_vfx_scene(
                commands,
                asset_server,
                "data/skill/magic_01.glb",
                source_pos,
                0.90,
                travel_duration_secs + 0.10,
                Some((effect_entity, Vec3::ZERO)),
            );
            spawn_timed_light(
                commands,
                target_pos + Vec3::new(0.0, 40.0, 0.0),
                None,
                total_duration_secs,
                total_duration_secs * 0.45,
                12000.0,
                2000.0,
                Color::srgb(1.0, 0.70, 0.40),
                280.0,
            );
        }
        DwMagicKind::Lightning => {
            spawn_timed_light(
                commands,
                source_pos,
                None,
                total_duration_secs,
                0.06,
                15000.0,
                3000.0,
                Color::srgb(0.72, 0.86, 1.0),
                280.0,
            );
            spawn_timed_light(
                commands,
                target_pos + Vec3::new(0.0, 18.0, 0.0),
                None,
                total_duration_secs,
                0.08,
                20000.0,
                3500.0,
                Color::srgb(0.72, 0.86, 1.0),
                340.0,
            );
        }
        DwMagicKind::FireBall => {
            spawn_skill_vfx_scene(
                commands,
                asset_server,
                "data/skill/fire_01.glb",
                source_pos,
                1.0,
                total_duration_secs,
                Some((effect_entity, Vec3::ZERO)),
            );
            spawn_skill_vfx_scene(
                commands,
                asset_server,
                "data/skill/magic_01.glb",
                source_pos,
                0.75,
                total_duration_secs,
                Some((effect_entity, Vec3::ZERO)),
            );
            spawn_timed_light(
                commands,
                source_pos,
                Some((effect_entity, Vec3::ZERO)),
                total_duration_secs,
                travel_duration_secs * 0.75,
                11000.0,
                1800.0,
                Color::srgb(1.0, 0.35, 0.18),
                180.0,
            );
        }
        DwMagicKind::Flame => {
            spawn_skill_vfx_scene(
                commands,
                asset_server,
                "data/skill/fire_01.glb",
                target_pos + Vec3::new(0.0, 18.0, 0.0),
                1.15,
                total_duration_secs,
                None,
            );
            if vfx_asset_exists("data/skill/storm_01.glb") {
                spawn_skill_vfx_scene(
                    commands,
                    asset_server,
                    "data/skill/storm_01.glb",
                    target_pos + Vec3::new(0.0, 26.0, 0.0),
                    1.0,
                    total_duration_secs,
                    None,
                );
            }
            spawn_timed_light(
                commands,
                target_pos + Vec3::new(0.0, 26.0, 0.0),
                None,
                total_duration_secs,
                0.20,
                16000.0,
                4200.0,
                Color::srgb(1.0, 0.40, 0.12),
                300.0,
            );
        }
        DwMagicKind::Twister => {
            spawn_skill_vfx_scene(
                commands,
                asset_server,
                "data/skill/storm_01.glb",
                target_pos + Vec3::new(0.0, 18.0, 0.0),
                1.20,
                total_duration_secs,
                Some((effect_entity, Vec3::new(0.0, 18.0, 0.0))),
            );
            spawn_timed_light(
                commands,
                target_pos + Vec3::new(0.0, 36.0, 0.0),
                Some((effect_entity, Vec3::new(0.0, 36.0, 0.0))),
                total_duration_secs,
                0.25,
                7000.0,
                1600.0,
                Color::srgb(0.55, 0.60, 0.72),
                260.0,
            );
        }
        DwMagicKind::EvilSpirit => {
            spawn_skill_vfx_scene(
                commands,
                asset_server,
                "data/skill/storm_01.glb",
                target_pos + Vec3::new(0.0, 20.0, 0.0),
                0.95,
                total_duration_secs,
                None,
            );
            spawn_timed_light(
                commands,
                target_pos + Vec3::new(0.0, 30.0, 0.0),
                None,
                total_duration_secs,
                0.15,
                9000.0,
                1400.0,
                Color::srgb(0.25, 0.45, 0.80),
                200.0,
            );
        }
        DwMagicKind::HellFire => {
            spawn_skill_vfx_scene(
                commands,
                asset_server,
                "data/skill/circle_01.glb",
                source_pos + Vec3::new(0.0, 4.0, 0.0),
                1.55,
                total_duration_secs,
                Some((caster_entity, Vec3::new(0.0, 4.0, 0.0))),
            );
            if vfx_asset_exists("data/skill/circle_02.glb") {
                spawn_skill_vfx_scene(
                    commands,
                    asset_server,
                    "data/skill/circle_02.glb",
                    source_pos + Vec3::new(0.0, 6.0, 0.0),
                    1.50,
                    total_duration_secs,
                    Some((caster_entity, Vec3::new(0.0, 6.0, 0.0))),
                );
            }
            spawn_skill_vfx_scene(
                commands,
                asset_server,
                "data/skill/fire_01.glb",
                source_pos + Vec3::new(0.0, 20.0, 0.0),
                1.05,
                total_duration_secs,
                Some((caster_entity, Vec3::new(0.0, 20.0, 0.0))),
            );
            spawn_timed_light(
                commands,
                source_pos + Vec3::new(0.0, 25.0, 0.0),
                Some((caster_entity, Vec3::new(0.0, 25.0, 0.0))),
                total_duration_secs,
                0.14,
                17000.0,
                2800.0,
                Color::srgb(1.0, 0.55, 0.25),
                320.0,
            );
        }
        DwMagicKind::AquaBeam => {
            let mut direction = target_pos - source_pos;
            direction.y = 0.0;
            direction = direction.normalize_or_zero();
            if direction.length_squared() <= f32::EPSILON {
                direction = Vec3::NEG_Z;
            }

            for segment in 0..20 {
                let segment_offset = direction * (segment as f32 * 50.0);
                queue_skill_vfx_scene(
                    commands,
                    "data/skill/ice_01.glb",
                    source_pos + Vec3::new(0.0, 12.0, 0.0) + segment_offset,
                    0.45,
                    0.26,
                    segment as f32 * 0.011,
                    None,
                );
            }

            spawn_skill_vfx_scene(
                commands,
                asset_server,
                "data/skill/storm_01.glb",
                target_pos + Vec3::new(0.0, 18.0, 0.0),
                0.90,
                total_duration_secs,
                None,
            );
            spawn_timed_light(
                commands,
                source_pos.lerp(target_pos, 0.5) + Vec3::new(0.0, 18.0, 0.0),
                None,
                total_duration_secs,
                0.10,
                14000.0,
                1800.0,
                Color::srgb(0.45, 0.72, 1.0),
                300.0,
            );
        }
        DwMagicKind::Inferno => {
            spawn_skill_vfx_scene(
                commands,
                asset_server,
                "data/skill/inferno_01.glb",
                target_pos + Vec3::new(0.0, 8.0, 0.0),
                1.25,
                total_duration_secs,
                None,
            );
            spawn_skill_vfx_scene(
                commands,
                asset_server,
                "data/skill/fire_01.glb",
                target_pos + Vec3::new(0.0, 20.0, 0.0),
                1.05,
                total_duration_secs,
                None,
            );
            spawn_timed_light(
                commands,
                target_pos + Vec3::new(0.0, 22.0, 0.0),
                None,
                total_duration_secs,
                0.10,
                18000.0,
                3200.0,
                Color::srgb(1.0, 0.35, 0.12),
                320.0,
            );
        }
        DwMagicKind::EnergyBall => {
            spawn_skill_vfx_scene(
                commands,
                asset_server,
                "data/skill/air_force.glb",
                source_pos,
                1.0,
                total_duration_secs,
                Some((effect_entity, Vec3::ZERO)),
            );
            spawn_timed_light(
                commands,
                source_pos,
                Some((effect_entity, Vec3::ZERO)),
                total_duration_secs,
                travel_duration_secs * 0.75,
                10000.0,
                1500.0,
                Color::srgb(0.30, 0.62, 1.0),
                170.0,
            );
        }
        DwMagicKind::IceStorm => {
            spawn_skill_vfx_scene(
                commands,
                asset_server,
                "data/skill/blizzard.glb",
                target_pos + Vec3::new(0.0, 36.0, 0.0),
                1.05,
                total_duration_secs,
                None,
            );
            spawn_timed_light(
                commands,
                target_pos + Vec3::new(0.0, 30.0, 0.0),
                None,
                total_duration_secs,
                0.25,
                13000.0,
                2600.0,
                Color::srgb(0.45, 0.62, 1.0),
                280.0,
            );
        }
    }
}

fn update_meteorite_effect(
    commands: &mut Commands,
    asset_server: &AssetServer,
    effect: &mut DwMagicRuntime,
    effect_transform: &mut Transform,
    rng: &mut rand::rngs::ThreadRng,
) {
    let travel_t = if effect.travel_duration_secs > 0.0 {
        (effect.elapsed_secs / effect.travel_duration_secs).clamp(0.0, 1.0)
    } else {
        1.0
    };

    let eased = travel_t * travel_t;
    let arc = (1.0 - travel_t).powi(2) * 220.0;
    let target_ground = effect.target_pos + Vec3::new(0.0, 12.0, 0.0);
    effect_transform.translation = effect.source_pos.lerp(target_ground, eased) + Vec3::Y * arc;

    while effect.emission_accumulator_secs >= 0.035 && !effect.impact_triggered {
        effect.emission_accumulator_secs -= 0.035;
        spawn_burst(
            commands,
            effect_transform.translation,
            0.0,
            5,
            0.28,
            SkillBurstEmitterConfig {
                lifetime_range: (0.08, 0.24),
                initial_velocity: Vec3::new(0.0, 20.0, 0.0),
                velocity_variance: Vec3::new(35.0, 25.0, 35.0),
                scale_range: (2.0, 5.0),
                scale_variance: 1.0,
                color_start: Vec4::new(1.0, 0.75, 0.45, 0.95),
                color_end: Vec4::new(1.0, 0.35, 0.10, 0.0),
                texture_path: "data/effect/spark_03.png".to_string(),
                additive: true,
                rotation_speed: 4.0,
            },
        );
    }

    if !effect.impact_triggered && effect.elapsed_secs >= effect.travel_duration_secs {
        effect.impact_triggered = true;
        spawn_skill_vfx_scene(
            commands,
            asset_server,
            "data/skill/blast_01.glb",
            target_ground,
            1.35,
            0.90,
            None,
        );
        spawn_skill_vfx_scene(
            commands,
            asset_server,
            "data/skill/storm_01.glb",
            target_ground + Vec3::new(0.0, 10.0, 0.0),
            1.15,
            0.95,
            None,
        );
        spawn_burst(
            commands,
            target_ground,
            0.0,
            28,
            0.65,
            SkillBurstEmitterConfig {
                lifetime_range: (0.10, 0.40),
                initial_velocity: Vec3::new(0.0, 110.0, 0.0),
                velocity_variance: Vec3::new(95.0, 70.0, 95.0),
                scale_range: (4.0, 12.0),
                scale_variance: 2.5,
                color_start: Vec4::new(1.0, 0.70, 0.42, 1.0),
                color_end: Vec4::new(1.0, 0.25, 0.05, 0.0),
                texture_path: "data/effect/spark_01.png".to_string(),
                additive: true,
                rotation_speed: 6.0,
            },
        );
        spawn_timed_light(
            commands,
            target_ground + Vec3::new(0.0, 20.0, 0.0),
            None,
            0.45,
            0.08,
            24000.0,
            0.0,
            Color::srgb(1.0, 0.55, 0.28),
            420.0,
        );
    }

    if effect.impact_triggered && effect.elapsed_secs < effect.total_duration_secs {
        let pulse_t = (effect.elapsed_secs * 12.0 + effect.random_phase * TAU)
            .sin()
            .abs();
        if rng.gen_range(0.0_f32..1.0) < 0.12 {
            spawn_timed_light(
                commands,
                effect.target_pos + Vec3::new(0.0, 18.0, 0.0),
                None,
                0.16,
                0.03,
                6000.0 + pulse_t * 2400.0,
                1200.0,
                Color::srgb(1.0, 0.45, 0.20),
                220.0,
            );
        }
    }
}

fn update_lightning_effect(
    commands: &mut Commands,
    asset_server: &AssetServer,
    effect: &mut DwMagicRuntime,
    rng: &mut rand::rngs::ThreadRng,
) {
    while effect.emission_accumulator_secs >= 0.045 {
        effect.emission_accumulator_secs -= 0.045;

        let offset = Vec3::new(
            rng.gen_range(-24.0_f32..24.0),
            rng.gen_range(-8.0_f32..22.0),
            rng.gen_range(-24.0_f32..24.0),
        );
        let source = effect.source_pos + offset * 0.25;
        let target = effect.target_pos + Vec3::new(0.0, 10.0, 0.0) + offset;
        let center = source.lerp(target, 0.5);
        let rotation = vfx_look_rotation(source, target, Vec3::NEG_Z);
        let scale = (source.distance(target) / 260.0).clamp(0.60, 2.20);

        spawn_skill_vfx_scene_with_rotation(
            commands,
            asset_server,
            "data/skill/laser_01.glb",
            center,
            scale,
            rotation,
            0.12,
            None,
        );

        if rng.gen_range(0.0_f32..1.0) < 0.50 {
            spawn_burst(
                commands,
                target,
                0.0,
                6,
                0.22,
                SkillBurstEmitterConfig {
                    lifetime_range: (0.05, 0.18),
                    initial_velocity: Vec3::new(0.0, 40.0, 0.0),
                    velocity_variance: Vec3::new(40.0, 30.0, 40.0),
                    scale_range: (2.5, 5.5),
                    scale_variance: 1.2,
                    color_start: Vec4::new(0.75, 0.90, 1.0, 1.0),
                    color_end: Vec4::new(0.45, 0.70, 1.0, 0.0),
                    texture_path: "data/effect/spark_02.png".to_string(),
                    additive: true,
                    rotation_speed: 5.5,
                },
            );
        }
    }

    if !effect.impact_triggered && effect.elapsed_secs >= effect.total_duration_secs * 0.92 {
        effect.impact_triggered = true;
        spawn_skill_vfx_scene(
            commands,
            asset_server,
            "data/skill/blast_01.glb",
            effect.target_pos + Vec3::new(0.0, 8.0, 0.0),
            0.95,
            0.28,
            None,
        );
    }
}

fn update_fire_ball_effect(
    commands: &mut Commands,
    asset_server: &AssetServer,
    effect: &mut DwMagicRuntime,
    effect_transform: &mut Transform,
    rng: &mut rand::rngs::ThreadRng,
) {
    update_projectile_motion(
        effect,
        effect_transform,
        60.0,
        effect.target_pos + Vec3::new(0.0, 12.0, 0.0),
    );

    while effect.emission_accumulator_secs >= 0.015 && !effect.impact_triggered {
        effect.emission_accumulator_secs -= 0.015;
        spawn_burst(
            commands,
            effect_transform.translation,
            0.0,
            4,
            0.22,
            SkillBurstEmitterConfig {
                lifetime_range: (0.05, 0.15),
                initial_velocity: Vec3::new(0.0, 18.0, 0.0),
                velocity_variance: Vec3::new(25.0, 20.0, 25.0),
                scale_range: (1.8, 4.0),
                scale_variance: 0.8,
                color_start: Vec4::new(1.0, 0.75, 0.50, 0.95),
                color_end: Vec4::new(1.0, 0.32, 0.10, 0.0),
                texture_path: "data/effect/spark_01.png".to_string(),
                additive: true,
                rotation_speed: 5.0,
            },
        );
    }

    if !effect.impact_triggered && effect.elapsed_secs >= effect.travel_duration_secs {
        effect.impact_triggered = true;
        spawn_skill_vfx_scene(
            commands,
            asset_server,
            "data/skill/blast_01.glb",
            effect.target_pos + Vec3::new(0.0, 8.0, 0.0),
            1.05,
            0.45,
            None,
        );
        spawn_burst(
            commands,
            effect.target_pos + Vec3::new(0.0, 10.0, 0.0),
            0.0,
            16,
            0.35,
            SkillBurstEmitterConfig {
                lifetime_range: (0.07, 0.28),
                initial_velocity: Vec3::new(0.0, 60.0, 0.0),
                velocity_variance: Vec3::new(55.0, 50.0, 55.0),
                scale_range: (2.5, 7.0),
                scale_variance: 1.5,
                color_start: Vec4::new(1.0, 0.82, 0.50, 1.0),
                color_end: Vec4::new(1.0, 0.35, 0.10, 0.0),
                texture_path: "data/effect/spark_01.png".to_string(),
                additive: true,
                rotation_speed: 6.0,
            },
        );
        spawn_timed_light(
            commands,
            effect.target_pos + Vec3::new(0.0, 20.0, 0.0),
            None,
            0.30,
            0.06,
            18000.0,
            0.0,
            Color::srgb(1.0, 0.40, 0.22),
            260.0,
        );
    }

    if effect.impact_triggered && rng.gen_range(0.0_f32..1.0) < 0.06 {
        spawn_burst(
            commands,
            effect.target_pos + Vec3::new(0.0, 12.0, 0.0),
            0.0,
            2,
            0.18,
            SkillBurstEmitterConfig {
                lifetime_range: (0.05, 0.12),
                initial_velocity: Vec3::new(0.0, 28.0, 0.0),
                velocity_variance: Vec3::new(20.0, 12.0, 20.0),
                scale_range: (1.5, 2.8),
                scale_variance: 0.5,
                color_start: Vec4::new(1.0, 0.72, 0.42, 0.8),
                color_end: Vec4::new(1.0, 0.28, 0.08, 0.0),
                texture_path: "data/effect/fire_01.png".to_string(),
                additive: true,
                rotation_speed: 3.0,
            },
        );
    }
}

fn update_flame_effect(
    commands: &mut Commands,
    asset_server: &AssetServer,
    effect: &mut DwMagicRuntime,
    rng: &mut rand::rngs::ThreadRng,
) {
    while effect.emission_accumulator_secs >= 0.04 {
        effect.emission_accumulator_secs -= 0.04;

        let angle = rng.gen_range(0.0_f32..TAU);
        let radius = rng.gen_range(0.0_f32..150.0);
        let pos = effect.target_pos
            + Vec3::new(
                radius * angle.cos(),
                rng.gen_range(6.0..48.0),
                radius * angle.sin(),
            );

        if rng.gen_range(0.0_f32..1.0) < 0.55 {
            queue_skill_vfx_scene(
                commands,
                "data/skill/fire_01.glb",
                pos,
                rng.gen_range(0.45_f32..0.95),
                0.35,
                rng.gen_range(0.0_f32..0.04),
                None,
            );
        }

        spawn_burst(
            commands,
            pos,
            0.0,
            4,
            0.25,
            SkillBurstEmitterConfig {
                lifetime_range: (0.08, 0.22),
                initial_velocity: Vec3::new(0.0, 48.0, 0.0),
                velocity_variance: Vec3::new(22.0, 22.0, 22.0),
                scale_range: (2.0, 5.0),
                scale_variance: 0.9,
                color_start: Vec4::new(1.0, 0.62, 0.22, 0.85),
                color_end: Vec4::new(0.8, 0.25, 0.08, 0.0),
                texture_path: "data/effect/fire_01.png".to_string(),
                additive: true,
                rotation_speed: 4.2,
            },
        );
    }

    if rng.gen_range(0.0_f32..1.0) < 0.09 {
        spawn_timed_light(
            commands,
            effect.target_pos + Vec3::new(0.0, 24.0, 0.0),
            None,
            0.20,
            0.06,
            9000.0,
            2800.0,
            Color::srgb(1.0, 0.40, 0.12),
            230.0,
        );
    }

    if !effect.impact_triggered && effect.elapsed_secs >= effect.total_duration_secs * 0.45 {
        effect.impact_triggered = true;
        spawn_skill_vfx_scene(
            commands,
            asset_server,
            "data/skill/blast_01.glb",
            effect.target_pos + Vec3::new(0.0, 10.0, 0.0),
            1.15,
            0.50,
            None,
        );
    }
}

fn update_twister_effect(
    commands: &mut Commands,
    asset_server: &AssetServer,
    effect: &mut DwMagicRuntime,
    rng: &mut rand::rngs::ThreadRng,
) {
    let swirl_angle = effect.auxiliary_accumulator_frames * 0.22;

    while effect.emission_accumulator_secs >= 0.035 {
        effect.emission_accumulator_secs -= 0.035;

        let radius = rng.gen_range(24.0_f32..95.0);
        let height = rng.gen_range(8.0_f32..90.0);
        let pos = effect.target_pos
            + Vec3::new(
                (swirl_angle + rng.gen_range(-0.4..0.4)).cos() * radius,
                height,
                (swirl_angle + rng.gen_range(-0.4..0.4)).sin() * radius,
            );

        spawn_burst(
            commands,
            pos,
            0.0,
            3,
            0.22,
            SkillBurstEmitterConfig {
                lifetime_range: (0.12, 0.28),
                initial_velocity: Vec3::new(0.0, 42.0, 0.0),
                velocity_variance: Vec3::new(18.0, 18.0, 18.0),
                scale_range: (2.5, 6.0),
                scale_variance: 0.9,
                color_start: Vec4::new(0.55, 0.62, 0.70, 0.75),
                color_end: Vec4::new(0.30, 0.34, 0.40, 0.0),
                texture_path: "data/effect/smoke_01.png".to_string(),
                additive: false,
                rotation_speed: 1.8,
            },
        );
    }

    if rng.gen_range(0.0_f32..1.0) < 0.14 {
        let strike_pos = effect.target_pos
            + Vec3::new(
                rng.gen_range(-70.0_f32..70.0),
                rng.gen_range(12.0_f32..65.0),
                rng.gen_range(-70.0_f32..70.0),
            );
        spawn_skill_vfx_scene(
            commands,
            asset_server,
            "data/skill/laser_01.glb",
            strike_pos,
            0.65,
            0.20,
            None,
        );
    }

    if rng.gen_range(0.0_f32..1.0) < 0.09 && vfx_asset_exists("data/skill/ground_stone.glb") {
        let ground_pos = effect.target_pos
            + Vec3::new(
                rng.gen_range(-95.0_f32..95.0),
                4.0,
                rng.gen_range(-95.0_f32..95.0),
            );
        spawn_skill_vfx_scene(
            commands,
            asset_server,
            "data/skill/ground_stone.glb",
            ground_pos,
            rng.gen_range(0.75_f32..1.10),
            0.55,
            None,
        );
    }
}

fn update_evil_spirit_effect(
    commands: &mut Commands,
    asset_server: &AssetServer,
    caster_transforms: &Query<&GlobalTransform>,
    effect: &mut DwMagicRuntime,
    frame_factor: f32,
    rng: &mut rand::rngs::ThreadRng,
) {
    let caster_anchor = caster_transforms
        .get(effect.caster_entity)
        .map(|transform| transform.translation())
        .unwrap_or(effect.target_pos);
    let anchor = caster_anchor + Vec3::new(0.0, EVIL_SPIRIT_TARGET_HEIGHT, 0.0);

    if !effect.evil_spirit_initialized {
        initialize_evil_spirit_states(effect, anchor);
    }

    while effect.emission_accumulator_secs >= 0.06 {
        effect.emission_accumulator_secs -= 0.06;
        effect.auxiliary_accumulator_frames += 1.0;

        for spirit in effect.evil_spirit_states.iter_mut() {
            if !spirit.active {
                continue;
            }

            if spirit.life_time <= 0.0 {
                spirit.active = false;
                continue;
            }

            if spirit.num_tails < (EVIL_SPIRIT_LIGHTNING_TAILS as u8) {
                spirit.num_tails += 1;
            }
            for tail_index in (1..=spirit.num_tails as usize).rev() {
                if tail_index >= spirit.tails.len() {
                    continue;
                }
                spirit.tails[tail_index] = spirit.tails[tail_index - 1];
            }
            spirit.tails[0] = spirit.position;

            move_humming_spirit(
                &mut spirit.position,
                &mut spirit.angle_x,
                &mut spirit.angle_z,
                &mut spirit.direction_x,
                &mut spirit.direction_z,
                anchor,
                EVIL_SPIRIT_TURN_RATE,
                frame_factor,
                rng,
            );

            let life_factor = (spirit.life_time / EVIL_SPIRIT_LIFETIME_SECONDS).clamp(0.0, 1.0);
            let alpha_scale = life_factor * 0.45;
            let core_scale = spirit.scale * (0.25 + alpha_scale);
            queue_skill_vfx_scene(
                commands,
                "data/skill/evil_spirit_2.glb",
                spirit.position,
                core_scale,
                0.16,
                0.0,
                None,
            );

            for tail_iter in 0..spirit.num_tails as usize {
                if tail_iter >= spirit.tails.len() {
                    continue;
                }
                let fade = 1.0 - (tail_iter as f32 / spirit.tails.len() as f32);
                queue_skill_vfx_scene(
                    commands,
                    "data/skill/evil_spirit_2.glb",
                    spirit.tails[tail_iter],
                    core_scale * (0.35 + fade * 0.45),
                    0.10 + fade * 0.05,
                    0.005 * tail_iter as f32,
                    None,
                );
            }

            if rng.gen_range(0.0_f32..1.0) < 0.28 {
                spawn_burst(
                    commands,
                    spirit.position,
                    0.0,
                    2,
                    0.18,
                    SkillBurstEmitterConfig {
                        lifetime_range: (0.05, 0.10),
                        initial_velocity: Vec3::new(0.0, 12.0, 0.0),
                        velocity_variance: Vec3::new(11.0, 9.0, 11.0),
                        scale_range: (1.2, 2.4),
                        scale_variance: 0.4,
                        color_start: Vec4::new(0.40, 0.65, 1.0, 0.8),
                        color_end: Vec4::new(0.12, 0.25, 0.55, 0.0),
                        texture_path: "data/effect/spark_02.png".to_string(),
                        additive: true,
                        rotation_speed: 4.6,
                    },
                );
            }
            spirit.life_time -= frame_factor / DW_BASE_FPS;
        }
    }

    if !effect.impact_triggered && effect.elapsed_secs >= effect.total_duration_secs * 0.78 {
        effect.impact_triggered = true;
        spawn_skill_vfx_scene(
            commands,
            asset_server,
            "data/skill/storm_01.glb",
            anchor + Vec3::new(0.0, 25.0, 0.0),
            1.05,
            0.42,
            None,
        );
    }
}

fn initialize_evil_spirit_states(effect: &mut DwMagicRuntime, anchor: Vec3) {
    let start_pos = anchor + Vec3::new(0.0, EVIL_SPIRIT_INITIAL_HEIGHT, 0.0);
    for i in 0..EVIL_SPIRIT_BOLT_COUNT {
        effect.evil_spirit_states[i].active = false;
    }

    let scales = [EVIL_SPIRIT_MAIN_SCALE, EVIL_SPIRIT_VISUAL_SCALE];
    let mut state_index = 0;
    for spirit_index in 0..4 {
        let base_angle = spirit_index as f32 * (TAU / 4.0);
        for scale in scales {
            effect.evil_spirit_states[state_index] = EvilSpiritState {
                position: start_pos,
                tails: [start_pos; EVIL_SPIRIT_LIGHTNING_TAILS + 1],
                num_tails: 0,
                life_time: EVIL_SPIRIT_LIFETIME_SECONDS,
                angle_x: 0.0,
                angle_z: base_angle + effect.random_phase,
                direction_x: 0.0,
                direction_z: 0.0,
                scale,
                active: true,
            };
            state_index += 1;
        }
    }

    effect.evil_spirit_initialized = true;
    effect.target_pos = anchor
        - Vec3::new(
            0.0,
            EVIL_SPIRIT_INITIAL_HEIGHT - EVIL_SPIRIT_TARGET_HEIGHT,
            0.0,
        );
}

#[allow(clippy::too_many_arguments)]
fn move_humming_spirit(
    position: &mut Vec3,
    angle_x: &mut f32,
    angle_z: &mut f32,
    direction_x: &mut f32,
    direction_z: &mut f32,
    target: Vec3,
    turn_rate: f32,
    frame_factor: f32,
    rng: &mut rand::rngs::ThreadRng,
) {
    let dx = target.x - position.x;
    let dz = target.z - position.z;
    let target_yaw = dx.atan2(-dz).to_degrees();
    *angle_z = turn_angle(*angle_z, target_yaw, turn_rate * frame_factor);

    let horizontal_dist = (dx * dx + dz * dz).sqrt();
    let target_pitch = (target.y - position.y).atan2(horizontal_dist).to_degrees();
    *angle_x = turn_angle(*angle_x, target_pitch, turn_rate * frame_factor);

    *direction_x += (rng.gen_range(0..32) as f32 - 16.0) * 0.2;
    *direction_z += (rng.gen_range(0..32) as f32 - 16.0) * 0.8;

    *angle_x += *direction_x * frame_factor;
    *angle_z += *direction_z * frame_factor;
    *direction_x *= 0.6;
    *direction_z *= 0.8;

    let yaw_rad = angle_z.to_radians();
    let pitch_rad = angle_x.to_radians();
    let forward = Vec3::new(
        yaw_rad.sin() * pitch_rad.cos(),
        pitch_rad.sin(),
        -yaw_rad.cos() * pitch_rad.cos(),
    );
    *position += forward * EVIL_SPIRIT_VELOCITY * frame_factor;
}

fn turn_angle(current: f32, target: f32, max_turn: f32) -> f32 {
    let mut diff = target - current;
    while diff > 180.0 {
        diff -= 360.0;
    }
    while diff < -180.0 {
        diff += 360.0;
    }

    if diff.abs() <= max_turn {
        target
    } else {
        current + diff.signum() * max_turn
    }
}

fn update_hell_fire_effect(
    commands: &mut Commands,
    asset_server: &AssetServer,
    caster_transforms: &Query<&GlobalTransform>,
    effect: &mut DwMagicRuntime,
    effect_transform: &mut Transform,
    rng: &mut rand::rngs::ThreadRng,
) {
    if let Ok(caster_transform) = caster_transforms.get(effect.caster_entity) {
        effect_transform.translation = caster_transform.translation();
        effect.target_pos = caster_transform.translation();
    }

    while effect.emission_accumulator_secs >= 0.11 {
        effect.emission_accumulator_secs -= 0.11;

        if vfx_asset_exists("data/skill/ground_stone.glb") {
            let angle = rng.gen_range(0.0_f32..TAU);
            let radius = rng.gen_range(20.0_f32..95.0);
            let ground_pos =
                effect.target_pos + Vec3::new(radius * angle.cos(), 3.0, radius * angle.sin());
            spawn_skill_vfx_scene(
                commands,
                asset_server,
                "data/skill/ground_stone.glb",
                ground_pos,
                rng.gen_range(0.75_f32..1.20),
                0.52,
                None,
            );
        }

        spawn_burst(
            commands,
            effect.target_pos + Vec3::new(0.0, 18.0, 0.0),
            0.0,
            6,
            0.30,
            SkillBurstEmitterConfig {
                lifetime_range: (0.08, 0.20),
                initial_velocity: Vec3::new(0.0, 40.0, 0.0),
                velocity_variance: Vec3::new(28.0, 24.0, 28.0),
                scale_range: (2.0, 5.0),
                scale_variance: 0.8,
                color_start: Vec4::new(1.0, 0.68, 0.30, 0.95),
                color_end: Vec4::new(1.0, 0.30, 0.08, 0.0),
                texture_path: "data/effect/fire_01.png".to_string(),
                additive: true,
                rotation_speed: 4.0,
            },
        );
    }

    if rng.gen_range(0.0_f32..1.0) < 0.10 {
        spawn_timed_light(
            commands,
            effect.target_pos + Vec3::new(0.0, 24.0, 0.0),
            None,
            0.16,
            0.04,
            7500.0,
            2200.0,
            Color::srgb(1.0, 0.55, 0.25),
            200.0,
        );
    }
}

fn update_aqua_beam_effect(
    commands: &mut Commands,
    asset_server: &AssetServer,
    effect: &mut DwMagicRuntime,
    rng: &mut rand::rngs::ThreadRng,
) {
    while effect.emission_accumulator_secs >= 0.05 {
        effect.emission_accumulator_secs -= 0.05;

        let mut direction = effect.target_pos - effect.source_pos;
        direction.y = 0.0;
        direction = direction.normalize_or_zero();
        if direction.length_squared() <= f32::EPSILON {
            direction = Vec3::NEG_Z;
        }

        let beam_progress = (effect.elapsed_secs / effect.total_duration_secs).clamp(0.0, 1.0);
        let pulse_pos =
            effect.source_pos + direction * (beam_progress * 700.0) + Vec3::new(0.0, 12.0, 0.0);
        queue_skill_vfx_scene(
            commands,
            "data/skill/ice_01.glb",
            pulse_pos,
            0.42,
            0.22,
            0.0,
            None,
        );

        if rng.gen_range(0.0_f32..1.0) < 0.5 {
            spawn_burst(
                commands,
                pulse_pos,
                0.0,
                3,
                0.20,
                SkillBurstEmitterConfig {
                    lifetime_range: (0.05, 0.16),
                    initial_velocity: Vec3::new(0.0, 22.0, 0.0),
                    velocity_variance: Vec3::new(20.0, 16.0, 20.0),
                    scale_range: (1.8, 3.8),
                    scale_variance: 0.6,
                    color_start: Vec4::new(0.50, 0.78, 1.0, 0.88),
                    color_end: Vec4::new(0.25, 0.50, 0.90, 0.0),
                    texture_path: "data/effect/spark_02.png".to_string(),
                    additive: true,
                    rotation_speed: 4.0,
                },
            );
        }
    }

    if !effect.impact_triggered && effect.elapsed_secs >= effect.total_duration_secs * 0.65 {
        effect.impact_triggered = true;
        spawn_skill_vfx_scene(
            commands,
            asset_server,
            "data/skill/storm_01.glb",
            effect.target_pos + Vec3::new(0.0, 16.0, 0.0),
            1.0,
            0.34,
            None,
        );
    }
}

fn update_inferno_effect(
    commands: &mut Commands,
    asset_server: &AssetServer,
    effect: &mut DwMagicRuntime,
    rng: &mut rand::rngs::ThreadRng,
) {
    if !effect.impact_triggered && effect.elapsed_secs >= 0.10 {
        effect.impact_triggered = true;
        for burst_index in 0..8 {
            let angle = burst_index as f32 * (TAU / 8.0) + effect.random_phase;
            let pos = effect.target_pos + Vec3::new(angle.cos() * 220.0, 10.0, angle.sin() * 220.0);
            queue_skill_vfx_scene(
                commands,
                "data/skill/blast_01.glb",
                pos,
                0.75,
                0.28,
                burst_index as f32 * 0.015,
                None,
            );
            if vfx_asset_exists("data/skill/ground_stone.glb") {
                queue_skill_vfx_scene(
                    commands,
                    "data/skill/ground_stone.glb",
                    pos + Vec3::new(0.0, 2.0, 0.0),
                    rng.gen_range(0.80_f32..1.15),
                    0.48,
                    burst_index as f32 * 0.020,
                    None,
                );
            }
            spawn_burst(
                commands,
                pos,
                burst_index as f32 * 0.010,
                8,
                0.30,
                SkillBurstEmitterConfig {
                    lifetime_range: (0.06, 0.24),
                    initial_velocity: Vec3::new(0.0, 65.0, 0.0),
                    velocity_variance: Vec3::new(42.0, 38.0, 42.0),
                    scale_range: (2.5, 6.5),
                    scale_variance: 1.0,
                    color_start: Vec4::new(1.0, 0.62, 0.25, 0.90),
                    color_end: Vec4::new(0.95, 0.20, 0.05, 0.0),
                    texture_path: "data/effect/fire_01.png".to_string(),
                    additive: true,
                    rotation_speed: 4.4,
                },
            );
        }
    }

    while effect.emission_accumulator_secs >= 0.05 {
        effect.emission_accumulator_secs -= 0.05;
        let jitter = Vec3::new(
            rng.gen_range(-60.0_f32..60.0),
            rng.gen_range(12.0_f32..48.0),
            rng.gen_range(-60.0_f32..60.0),
        );
        spawn_burst(
            commands,
            effect.target_pos + jitter,
            0.0,
            4,
            0.20,
            SkillBurstEmitterConfig {
                lifetime_range: (0.06, 0.16),
                initial_velocity: Vec3::new(0.0, 24.0, 0.0),
                velocity_variance: Vec3::new(18.0, 18.0, 18.0),
                scale_range: (1.6, 3.4),
                scale_variance: 0.6,
                color_start: Vec4::new(1.0, 0.60, 0.20, 0.82),
                color_end: Vec4::new(0.95, 0.20, 0.05, 0.0),
                texture_path: "data/effect/spark_01.png".to_string(),
                additive: true,
                rotation_speed: 3.8,
            },
        );
    }

    if rng.gen_range(0.0_f32..1.0) < 0.08 {
        spawn_skill_vfx_scene(
            commands,
            asset_server,
            "data/skill/flashing.glb",
            effect.target_pos + Vec3::new(0.0, 26.0, 0.0),
            0.42,
            0.12,
            None,
        );
    }
}

fn update_energy_ball_effect(
    commands: &mut Commands,
    asset_server: &AssetServer,
    effect: &mut DwMagicRuntime,
    effect_transform: &mut Transform,
    rng: &mut rand::rngs::ThreadRng,
) {
    update_projectile_motion(
        effect,
        effect_transform,
        35.0,
        effect.target_pos + Vec3::new(0.0, 12.0, 0.0),
    );

    while effect.emission_accumulator_secs >= 0.018 && !effect.impact_triggered {
        effect.emission_accumulator_secs -= 0.018;
        spawn_burst(
            commands,
            effect_transform.translation,
            0.0,
            4,
            0.22,
            SkillBurstEmitterConfig {
                lifetime_range: (0.05, 0.16),
                initial_velocity: Vec3::new(0.0, 14.0, 0.0),
                velocity_variance: Vec3::new(24.0, 18.0, 24.0),
                scale_range: (1.8, 4.0),
                scale_variance: 0.8,
                color_start: Vec4::new(0.45, 0.72, 1.0, 0.92),
                color_end: Vec4::new(0.25, 0.50, 1.0, 0.0),
                texture_path: "data/effect/spark_02.png".to_string(),
                additive: true,
                rotation_speed: 5.0,
            },
        );
    }

    if !effect.impact_triggered && effect.elapsed_secs >= effect.travel_duration_secs {
        effect.impact_triggered = true;
        spawn_skill_vfx_scene(
            commands,
            asset_server,
            "data/skill/storm_01.glb",
            effect.target_pos + Vec3::new(0.0, 14.0, 0.0),
            0.95,
            0.42,
            None,
        );
        spawn_burst(
            commands,
            effect.target_pos + Vec3::new(0.0, 14.0, 0.0),
            0.0,
            14,
            0.32,
            SkillBurstEmitterConfig {
                lifetime_range: (0.06, 0.20),
                initial_velocity: Vec3::new(0.0, 55.0, 0.0),
                velocity_variance: Vec3::new(45.0, 40.0, 45.0),
                scale_range: (2.2, 5.5),
                scale_variance: 1.1,
                color_start: Vec4::new(0.62, 0.82, 1.0, 0.95),
                color_end: Vec4::new(0.28, 0.50, 1.0, 0.0),
                texture_path: "data/effect/spark_02.png".to_string(),
                additive: true,
                rotation_speed: 5.8,
            },
        );
        spawn_timed_light(
            commands,
            effect.target_pos + Vec3::new(0.0, 22.0, 0.0),
            None,
            0.26,
            0.06,
            16000.0,
            0.0,
            Color::srgb(0.35, 0.62, 1.0),
            250.0,
        );
    }

    if effect.impact_triggered && rng.gen_range(0.0_f32..1.0) < 0.07 {
        spawn_burst(
            commands,
            effect.target_pos + Vec3::new(0.0, 14.0, 0.0),
            0.0,
            2,
            0.18,
            SkillBurstEmitterConfig {
                lifetime_range: (0.05, 0.12),
                initial_velocity: Vec3::new(0.0, 20.0, 0.0),
                velocity_variance: Vec3::new(15.0, 12.0, 15.0),
                scale_range: (1.3, 2.5),
                scale_variance: 0.4,
                color_start: Vec4::new(0.55, 0.80, 1.0, 0.70),
                color_end: Vec4::new(0.30, 0.50, 0.95, 0.0),
                texture_path: "data/effect/spark_02.png".to_string(),
                additive: true,
                rotation_speed: 3.2,
            },
        );
    }
}

fn update_ice_storm_effect(
    commands: &mut Commands,
    asset_server: &AssetServer,
    effect: &mut DwMagicRuntime,
    rng: &mut rand::rngs::ThreadRng,
) {
    while effect.emission_accumulator_secs >= 0.16 {
        effect.emission_accumulator_secs -= 0.16;

        let angle = rng.gen_range(0.0_f32..TAU);
        let radius = rng.gen_range(0.0_f32..220.0);
        let ground_pos =
            effect.target_pos + Vec3::new(radius * angle.cos(), 8.0, radius * angle.sin());
        let drop_pos = ground_pos + Vec3::new(0.0, rng.gen_range(180.0_f32..340.0), 0.0);

        spawn_skill_vfx_scene(
            commands,
            asset_server,
            "data/skill/blizzard.glb",
            drop_pos,
            rng.gen_range(0.70_f32..1.15),
            0.35,
            None,
        );

        queue_skill_vfx_scene(
            commands,
            "data/skill/ice_01.glb",
            ground_pos,
            rng.gen_range(0.80_f32..1.05),
            0.40,
            0.28,
            None,
        );

        spawn_burst(
            commands,
            ground_pos,
            0.28,
            10,
            0.45,
            SkillBurstEmitterConfig {
                lifetime_range: (0.08, 0.30),
                initial_velocity: Vec3::new(0.0, 80.0, 0.0),
                velocity_variance: Vec3::new(55.0, 50.0, 55.0),
                scale_range: (2.0, 6.0),
                scale_variance: 1.0,
                color_start: Vec4::new(0.65, 0.82, 1.0, 0.95),
                color_end: Vec4::new(0.45, 0.65, 0.95, 0.0),
                texture_path: "data/effect/spark_02.png".to_string(),
                additive: true,
                rotation_speed: 4.6,
            },
        );

        if rng.gen_range(0.0_f32..1.0) < 0.55 {
            spawn_timed_light(
                commands,
                ground_pos + Vec3::new(0.0, 20.0, 0.0),
                None,
                0.24,
                0.07,
                12000.0,
                1400.0,
                Color::srgb(0.50, 0.68, 1.0),
                240.0,
            );
        }
    }
}

fn update_projectile_motion(
    effect: &DwMagicRuntime,
    effect_transform: &mut Transform,
    arc_height: f32,
    target: Vec3,
) {
    let travel_t = if effect.travel_duration_secs > 0.0 {
        (effect.elapsed_secs / effect.travel_duration_secs).clamp(0.0, 1.0)
    } else {
        1.0
    };
    let eased = travel_t * travel_t * (3.0 - 2.0 * travel_t);
    let parabola = (1.0 - (2.0 * travel_t - 1.0).powi(2)).max(0.0);

    effect_transform.translation =
        effect.source_pos.lerp(target, eased) + Vec3::new(0.0, parabola * arc_height, 0.0);
}

fn spawn_timed_light(
    commands: &mut Commands,
    position: Vec3,
    follow: Option<(Entity, Vec3)>,
    lifetime: f32,
    peak_time: f32,
    peak_intensity: f32,
    base_intensity: f32,
    color: Color,
    range: f32,
) {
    let mut entity = commands.spawn((
        SkillVfx,
        SkillTimedLight {
            elapsed: 0.0,
            lifetime: lifetime.max(0.05),
            peak_time: peak_time.clamp(0.0, lifetime.max(0.05)),
            peak_intensity,
            base_intensity,
            color,
            range,
        },
        PointLight {
            intensity: 0.0,
            range,
            shadows_enabled: false,
            ..default()
        },
        Transform::from_translation(position),
        Visibility::Hidden,
    ));

    if let Some((target, offset)) = follow {
        entity.insert(SkillVfxFollow { target, offset });
    }
}

fn spawn_burst(
    commands: &mut Commands,
    position: Vec3,
    delay: f32,
    burst_count: u32,
    lifetime_after_burst: f32,
    emitter_config: SkillBurstEmitterConfig,
) {
    commands.spawn((
        SkillVfx,
        SkillImpactBurst {
            delay: delay.max(0.0),
            elapsed: 0.0,
            fired: false,
            burst_count,
            emitter_config,
            lifetime_after_burst,
        },
        Transform::from_translation(position),
    ));
}

fn is_dw_magic_profile(profile: SkillVfxProfile) -> bool {
    matches!(
        profile,
        SkillVfxProfile::Meteorite
            | SkillVfxProfile::Lightning
            | SkillVfxProfile::FireBall
            | SkillVfxProfile::Flame
            | SkillVfxProfile::Twister
            | SkillVfxProfile::EvilSpirit
            | SkillVfxProfile::HellFire
            | SkillVfxProfile::AquaBeam
            | SkillVfxProfile::Inferno
            | SkillVfxProfile::EnergyBall
            | SkillVfxProfile::IceStorm
    )
}
