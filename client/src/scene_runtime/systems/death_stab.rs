use crate::infra::assets::{asset_path_exists, resolve_asset_path};
use crate::lightning_sprite_2d::{LightningSprite2dMaterial, LightningSprite2dParams};
use crate::scene_runtime::components::*;
use bevy::gltf::Gltf;
use bevy::image::{ImageAddressMode, ImageLoaderSettings, ImageSampler, ImageSamplerDescriptor};
use bevy::light::{NotShadowCaster, NotShadowReceiver};
use bevy::math::primitives::Rectangle;
use bevy::pbr::MeshMaterial3d;
use bevy::prelude::*;
use bevy::sprite_render::MeshMaterial2d;
use bevy::time::Timer;
use bevy::camera::ClearColorConfig;
use rand::Rng;
use std::time::Duration;

// ============================================================================
// Constants (from C# DeathStabEffect.cs timing model)
// ============================================================================

const DEATH_STAB_TOTAL_LIFE_FRAMES: f32 = 20.0;
const DEATH_STAB_REFERENCE_FPS: f32 = 25.0;
const DEATH_STAB_MAX_ANIMATION_FACTOR: f32 = 3.0;
const DEATH_STAB_SOUND_LIFE_INT: i32 = 12;
const DEATH_STAB_IMPACT_LIFE_INT: i32 = 10;
const DEATH_STAB_HURT_FRAMES: u8 = 35;
const DEATH_STAB_MAX_ACTIVE_LIGHTNING_ARCS: usize = 12;
const DEATH_STAB_ENERGY_WINDOW_MIN: i32 = 12;
const DEATH_STAB_ENERGY_WINDOW_MAX: i32 = 18;
const DEATH_STAB_SPIKE_WINDOW_MIN: i32 = 8;
const DEATH_STAB_SPIKE_WINDOW_MAX: i32 = 14;
const DEATH_STAB_ENERGY_SPAWN_DISTANCE: f32 = 600.0;
const DEATH_STAB_ENERGY_TARGET_DISTANCE: f32 = 300.0;
const DEATH_STAB_ENERGY_RANDOM_RADIUS: f32 = 300.0;
const DEATH_STAB_ENERGY_PARTICLE_LIFE_FRAMES: f32 = 20.0;
const DEATH_STAB_SPIKE_BASE_DISTANCE: f32 = 100.0;
const DEATH_STAB_SPIKE_LIFE_FRAMES: f32 = 10.0;
const DEATH_STAB_ENERGY_PARTICLES_PER_SPAWN: usize = 3;
const DEATH_STAB_SPIKES_PER_SPAWN: usize = 2;
const DEATH_STAB_CHARGE_LIGHT_INTENSITY: f32 = 8_000.0;
const DEATH_STAB_IMPACT_LIGHT_INTENSITY: f32 = 8_000.0;
const DEATH_STAB_LIGHTNING_FLASH_SECS: f32 = 0.06;
const DEATH_STAB_LIGHTNING_INTENSITY: f32 = 3.4;
const TERRAIN_SIZE: f32 = 256.0;
const LIGHTNING_TEX_WIDTH: f32 = 256.0;
const LIGHTNING_TEX_HEIGHT: f32 = 32.0;

#[derive(Default)]
pub struct DeathStabLightningAssets {
    mesh: Option<Handle<Mesh>>,
    material: Option<Handle<LightningSprite2dMaterial>>,
}

// ============================================================================
// Helper functions
// ============================================================================

fn fps_animation_factor(dt: f32) -> f32 {
    let fps = (1.0 / dt.max(0.000_1)).max(0.01);
    (DEATH_STAB_REFERENCE_FPS / fps).min(DEATH_STAB_MAX_ANIMATION_FACTOR)
}

fn rand_fps_check<R: Rng + ?Sized>(
    reference_frames: i32,
    fps_animation_factor: f32,
    rng: &mut R,
) -> bool {
    let animation_factor = fps_animation_factor.min(1.0);
    let chance = if reference_frames <= 1 {
        animation_factor
    } else {
        (1.0 / reference_frames as f32) * animation_factor
    }
    .clamp(0.0, 1.0);

    rng.gen_range(0.0..=1.0) <= chance
}

fn frames_to_seconds(frames: f32) -> f32 {
    (frames / DEATH_STAB_REFERENCE_FPS).max(0.05)
}

fn vfx_asset_exists(path: &str) -> bool {
    asset_path_exists(path)
}

fn charge_asset() -> Option<&'static str> {
    // C#: ResolveModelPath("RidingSpear", "Skill/", "Skill/RidingSpear01.bmd")
    if vfx_asset_exists("data/skill/riding_spear_01.glb") {
        Some("data/skill/riding_spear_01.glb")
    } else if vfx_asset_exists("data/skill/riding_spear.glb") {
        Some("data/skill/riding_spear.glb")
    } else if vfx_asset_exists("data/skill/m_piercing.glb") {
        Some("data/skill/m_piercing.glb")
    } else if vfx_asset_exists("data/skill/piercing.glb") {
        Some("data/skill/piercing.glb")
    } else {
        None
    }
}

fn spear_asset() -> Option<&'static str> {
    // C#: ResolveModelPath("Spear", "Item/", "Item/Spear02.bmd")
    // Prefer Spear02, fallback to Spear01.
    if vfx_asset_exists("data/item/spear_02.glb") {
        Some("data/item/spear_02.glb")
    } else if vfx_asset_exists("data/item/spear_01.glb") {
        Some("data/item/spear_01.glb")
    } else {
        None
    }
}

fn lightning_texture_asset() -> Option<&'static str> {
    if vfx_asset_exists("data/effect/joint_thunder_01.png") {
        Some("data/effect/joint_thunder_01.png")
    } else if vfx_asset_exists("data/effect/thunder_01.png") {
        Some("data/effect/thunder_01.png")
    } else if vfx_asset_exists("data/effect/eff_lighting.png") {
        Some("data/effect/eff_lighting.png")
    } else {
        None
    }
}

/// BFS check whether subtree contains any `MeshMaterial3d<StandardMaterial>`.
fn subtree_contains_material(
    root: Entity,
    children_query: &Query<&Children>,
    material_query: &Query<(), With<MeshMaterial3d<StandardMaterial>>>,
) -> bool {
    let mut queue = vec![root];
    while let Some(entity) = queue.pop() {
        if material_query.contains(entity) {
            return true;
        }
        if let Ok(children) = children_query.get(entity) {
            queue.extend(children.iter());
        }
    }
    false
}

/// BFS to find all `AnimationPlayer` entities in a subtree.
fn find_animation_players_in_subtree(
    root: Entity,
    children_query: &Query<&Children>,
    player_presence: &Query<(), With<AnimationPlayer>>,
) -> Vec<Entity> {
    let mut result = Vec::new();
    let mut queue = vec![root];
    while let Some(entity) = queue.pop() {
        if player_presence.contains(entity) {
            result.push(entity);
        }
        if let Ok(children) = children_query.get(entity) {
            queue.extend(children.iter());
        }
    }
    result
}

fn find_descendant_named(
    root: Entity,
    candidates: &[&str],
    children_query: &Query<&Children>,
    name_query: &Query<&Name>,
) -> Option<Entity> {
    for candidate in candidates {
        let mut queue = vec![root];
        while let Some(entity) = queue.pop() {
            if let Ok(name) = name_query.get(entity) {
                if name.as_str().eq_ignore_ascii_case(candidate) {
                    return Some(entity);
                }
            }
            if let Ok(children) = children_query.get(entity) {
                queue.extend(children.iter());
            }
        }
    }
    None
}

fn resolve_target_position(
    timeline: &mut DeathStabTimeline,
    global_transforms: &Query<&GlobalTransform>,
) -> Vec3 {
    if let Some(target_entity) = timeline.target_entity {
        if let Ok(target_gt) = global_transforms.get(target_entity) {
            timeline.target_pos = target_gt.translation();
        }
    }
    timeline.target_pos
}

fn resolve_weapon_tip_position(
    timeline: &mut DeathStabTimeline,
    caster_pos: Vec3,
    global_transforms: &Query<&GlobalTransform>,
    children_query: &Query<&Children>,
    name_query: &Query<&Name>,
) -> Vec3 {
    if timeline.weapon_hand_entity.is_none() {
        timeline.weapon_hand_entity = find_descendant_named(
            timeline.caster_entity,
            &[
                "Bip01 R Hand",
                "Bip01 R Forearm",
                "Bip01 L Hand",
                "Bip01 L Forearm",
            ],
            children_query,
            name_query,
        );
    }
    if timeline.weapon_tip_entity.is_none() {
        timeline.weapon_tip_entity = find_descendant_named(
            timeline.caster_entity,
            &[
                "Bip01 R Finger02",
                "Bip01 R Finger01",
                "Bip01 R Finger0",
                "Bip01 L Finger02",
                "Bip01 L Finger01",
                "Bip01 L Finger0",
            ],
            children_query,
            name_query,
        )
        .or(timeline.weapon_hand_entity);
    }

    if let Some(tip_entity) = timeline.weapon_tip_entity {
        if let Ok(tip_gt) = global_transforms.get(tip_entity) {
            return tip_gt.translation();
        }
    }
    if let Some(hand_entity) = timeline.weapon_hand_entity {
        if let Ok(hand_gt) = global_transforms.get(hand_entity) {
            return hand_gt.translation() + timeline.forward_xz * 30.0;
        }
    }

    caster_pos + timeline.forward_xz * DEATH_STAB_ENERGY_TARGET_DISTANCE + Vec3::Y * 120.0
}

fn fallback_lightning_anchor_points(center: Vec3) -> [Vec3; 14] {
    [
        center + Vec3::new(0.0, 74.0, 0.0),
        center + Vec3::new(0.0, 58.0, 12.0),
        center + Vec3::new(0.0, 58.0, -12.0),
        center + Vec3::new(18.0, 52.0, 0.0),
        center + Vec3::new(-18.0, 52.0, 0.0),
        center + Vec3::new(28.0, 30.0, 8.0),
        center + Vec3::new(-28.0, 30.0, 8.0),
        center + Vec3::new(20.0, 6.0, 4.0),
        center + Vec3::new(-20.0, 6.0, 4.0),
        center + Vec3::new(0.0, 40.0, 18.0),
        center + Vec3::new(0.0, 40.0, -18.0),
        center + Vec3::new(0.0, 22.0, 0.0),
        center + Vec3::new(0.0, 95.0, 0.0),
        center + Vec3::new(0.0, -30.0, 0.0),
    ]
}

fn apply_emissive_recursive(
    root: Entity,
    color: Color,
    children_query: &Query<&Children>,
    material_query: &Query<&MeshMaterial3d<StandardMaterial>>,
    materials: &mut Assets<StandardMaterial>,
) {
    let mut queue = vec![root];
    while let Some(entity) = queue.pop() {
        if let Ok(mat_handle) = material_query.get(entity) {
            if let Some(mat) = materials.get_mut(&mat_handle.0) {
                mat.base_color = color;
            }
        }
        if let Ok(children) = children_query.get(entity) {
            queue.extend(children.iter());
        }
    }
}

// ============================================================================
// Spawn functions
// ============================================================================

/// Spawn an energy particle with lerp movement and animation.
fn spawn_energy_particle(
    commands: &mut Commands,
    asset_server: &AssetServer,
    glb_path: &str,
    start_pos: Vec3,
    target_pos: Vec3,
    target_entity: Option<Entity>,
    uniform_scale: f32,
    rotation: Quat,
    ttl_seconds: f32,
) -> Entity {
    let resolved_glb_path = resolve_asset_path(glb_path);
    let scene_handle: Handle<Scene> = asset_server.load(format!("{resolved_glb_path}#Scene0"));
    let gltf_handle: Handle<Gltf> = asset_server.load(resolved_glb_path.clone());
    commands
        .spawn((
            SceneRoot(scene_handle),
            Transform::from_translation(start_pos)
                .with_rotation(rotation)
                .with_scale(Vec3::splat(uniform_scale)),
            Visibility::Hidden,
            RuntimeSceneEntity,
            DeathStabVfxParticle,
            DeathStabEnergyParticle {
                start_pos,
                target_pos,
                target_entity,
                max_lifetime_secs: ttl_seconds,
                elapsed_secs: 0.0,
            },
            DeathStabAnimationSource {
                gltf_handle,
                playback_speed: 5.0,
            },
            SkillVfxAutoLifetime {
                timer: Timer::from_seconds(ttl_seconds, TimerMode::Once),
            },
        ))
        .id()
}

/// Spawn a spike particle with fade-out and animation.
fn spawn_spike_particle(
    commands: &mut Commands,
    asset_server: &AssetServer,
    glb_path: &str,
    position: Vec3,
    uniform_scale: f32,
    rotation: Quat,
    ttl_seconds: f32,
) -> Entity {
    let resolved_glb_path = resolve_asset_path(glb_path);
    let scene_handle: Handle<Scene> = asset_server.load(format!("{resolved_glb_path}#Scene0"));
    let gltf_handle: Handle<Gltf> = asset_server.load(resolved_glb_path.clone());
    commands
        .spawn((
            SceneRoot(scene_handle),
            Transform::from_translation(position)
                .with_rotation(rotation)
                .with_scale(Vec3::splat(uniform_scale)),
            Visibility::Hidden,
            RuntimeSceneEntity,
            DeathStabVfxParticle,
            DeathStabSpikeParticle {
                max_lifetime_secs: ttl_seconds,
                elapsed_secs: 0.0,
            },
            DeathStabAnimationSource {
                gltf_handle,
                playback_speed: 4.0,
            },
            SkillVfxAutoLifetime {
                timer: Timer::from_seconds(ttl_seconds, TimerMode::Once),
            },
        ))
        .id()
}

// ============================================================================
// Spawn function
// ============================================================================

/// Spawn the DeathStab (Skill 43) three-phase VFX effect.
///
/// Returns the timeline entity so callers can attach additional markers.
pub fn spawn_death_stab_vfx(
    commands: &mut Commands,
    caster_entity: Entity,
    caster_pos: Vec3,
    caster_rotation: Quat,
    target_pos: Vec3,
) -> Entity {
    spawn_death_stab_vfx_tracked(
        commands,
        caster_entity,
        None,
        None,
        None,
        caster_pos,
        caster_rotation,
        target_pos,
    )
}

/// Spawn DeathStab with optional runtime tracking anchors (target and weapon bones).
pub fn spawn_death_stab_vfx_tracked(
    commands: &mut Commands,
    caster_entity: Entity,
    target_entity: Option<Entity>,
    weapon_hand_entity: Option<Entity>,
    weapon_tip_entity: Option<Entity>,
    caster_pos: Vec3,
    caster_rotation: Quat,
    target_pos: Vec3,
) -> Entity {
    let mut forward_xz = caster_rotation.mul_vec3(Vec3::NEG_Z);
    forward_xz.y = 0.0;
    if forward_xz.length_squared() <= f32::EPSILON {
        forward_xz = Vec3::NEG_Z;
    } else {
        forward_xz = forward_xz.normalize();
    }

    let charge_light_entity = commands
        .spawn((
            RuntimeSceneEntity,
            PointLight {
                intensity: 0.0,
                range: 220.0,
                color: Color::srgb(0.65, 0.85, 1.0),
                shadows_enabled: false,
                ..default()
            },
            Transform::from_translation(
                caster_pos + forward_xz * DEATH_STAB_ENERGY_TARGET_DISTANCE + Vec3::Y * 120.0,
            ),
            GlobalTransform::default(),
        ))
        .id();

    let impact_light_entity = commands
        .spawn((
            RuntimeSceneEntity,
            PointLight {
                intensity: 0.0,
                range: 200.0,
                color: Color::srgb(0.8, 0.95, 1.0),
                shadows_enabled: false,
                ..default()
            },
            Transform::from_translation(target_pos + Vec3::Y * 80.0),
            GlobalTransform::default(),
        ))
        .id();

    debug!(
        "Death Stab dimensions -> rear_spawn={} tip_distance={} random_radius={} spike_base={} (rear->front enabled)",
        DEATH_STAB_ENERGY_SPAWN_DISTANCE,
        DEATH_STAB_ENERGY_TARGET_DISTANCE,
        DEATH_STAB_ENERGY_RANDOM_RADIUS,
        DEATH_STAB_SPIKE_BASE_DISTANCE
    );

    commands
        .spawn((
            RuntimeSceneEntity,
            DeathStabTimeline {
                caster_entity,
                target_entity,
                weapon_hand_entity,
                weapon_tip_entity,
                caster_start_pos: caster_pos,
                target_pos,
                forward_xz,
                spear_rotation: Quat::from_rotation_arc(Vec3::Z, forward_xz),
                life_frames: DEATH_STAB_TOTAL_LIFE_FRAMES,
                last_processed_life_int: DEATH_STAB_TOTAL_LIFE_FRAMES as i32 + 1,
                sound_played: false,
                impact_applied: false,
                charge_light_entity,
                impact_light_entity,
                elapsed_seconds: 0.0,
            },
        ))
        .id()
}

// ============================================================================
// Timeline update system
// ============================================================================

pub fn update_death_stab_timeline(
    mut commands: Commands,
    time: Res<Time>,
    asset_server: Res<AssetServer>,
    global_transforms: Query<&GlobalTransform>,
    children_query: Query<&Children>,
    name_query: Query<&Name>,
    mut point_lights: Query<(&mut PointLight, &mut Transform)>,
    mut timelines: Query<(Entity, &mut DeathStabTimeline)>,
) {
    let dt = time.delta_secs();
    let factor = fps_animation_factor(dt);
    let charge_glb = charge_asset();
    let spear_glb = spear_asset();
    let energy_ttl = frames_to_seconds(DEATH_STAB_ENERGY_PARTICLE_LIFE_FRAMES);
    let spike_ttl = frames_to_seconds(DEATH_STAB_SPIKE_LIFE_FRAMES);
    let mut rng = rand::thread_rng();

    for (entity, mut timeline) in &mut timelines {
        timeline.elapsed_seconds += dt;

        let caster_pos = global_transforms
            .get(timeline.caster_entity)
            .map(|gt| gt.translation())
            .unwrap_or(timeline.caster_start_pos);
        let target_pos = resolve_target_position(&mut timeline, &global_transforms);
        let weapon_tip_pos = resolve_weapon_tip_position(
            &mut timeline,
            caster_pos,
            &global_transforms,
            &children_query,
            &name_query,
        );
        let life_int = timeline.life_frames.floor() as i32;

        // Update charge light
        if let Ok((mut charge_light, mut charge_transform)) =
            point_lights.get_mut(timeline.charge_light_entity)
        {
            let pulse = 0.8 + 0.2 * (timeline.elapsed_seconds * 14.0).sin();
            let charge_active =
                (DEATH_STAB_SPIKE_WINDOW_MIN..=DEATH_STAB_ENERGY_WINDOW_MAX).contains(&life_int);
            charge_transform.translation = weapon_tip_pos;
            charge_light.color = Color::srgb(0.65, 0.85, 1.0);
            charge_light.intensity = if charge_active {
                DEATH_STAB_CHARGE_LIGHT_INTENSITY * pulse
            } else {
                0.0
            };
            charge_light.range = if charge_active { 220.0 } else { 140.0 };
        }

        // Update impact light
        if let Ok((mut impact_light, mut impact_transform)) =
            point_lights.get_mut(timeline.impact_light_entity)
        {
            impact_transform.translation = target_pos + Vec3::Y * 80.0;
            impact_light.color = Color::srgb(0.8, 0.95, 1.0);
            if timeline.impact_applied {
                let impact_alpha =
                    (timeline.life_frames / DEATH_STAB_SPIKE_LIFE_FRAMES).clamp(0.0, 1.0);
                impact_light.intensity = DEATH_STAB_IMPACT_LIGHT_INTENSITY * impact_alpha;
                impact_light.range = 200.0 + 40.0 * (1.0 - impact_alpha);
            } else {
                impact_light.intensity = 0.0;
                impact_light.range = 200.0;
            }
        }

        // Process frame-by-frame events
        while timeline.last_processed_life_int > life_int {
            timeline.last_processed_life_int -= 1;
            let current_life = timeline.last_processed_life_int;

            // Sound cue at lifeInt 12
            if current_life == DEATH_STAB_SOUND_LIFE_INT && !timeline.sound_played {
                timeline.sound_played = true;
                debug!("Death Stab (skill 43): lifeInt 12 -> SOUND_SKILL_SWORD2 cue");
            }

            // Phase 1: Energy charge particles (lifeInt 12..18)
            if (DEATH_STAB_ENERGY_WINDOW_MIN..=DEATH_STAB_ENERGY_WINDOW_MAX).contains(&current_life)
                && rand_fps_check(1, factor, &mut rng)
            {
                if let Some(charge_path) = charge_glb {
                    for _ in 0..DEATH_STAB_ENERGY_PARTICLES_PER_SPAWN {
                        // C#: Random.Next(-300, 300) independently for X and Y (rectangular)
                        let spread = Vec3::new(
                            rng.gen_range(
                                -DEATH_STAB_ENERGY_RANDOM_RADIUS..DEATH_STAB_ENERGY_RANDOM_RADIUS,
                            ),
                            0.0,
                            rng.gen_range(
                                -DEATH_STAB_ENERGY_RANDOM_RADIUS..DEATH_STAB_ENERGY_RANDOM_RADIUS,
                            ),
                        );
                        let rear_origin = caster_pos + Vec3::Y * 120.0 + spread
                            - timeline.forward_xz * DEATH_STAB_ENERGY_SPAWN_DISTANCE;
                        spawn_energy_particle(
                            &mut commands,
                            &asset_server,
                            charge_path,
                            rear_origin,
                            weapon_tip_pos,
                            timeline.weapon_tip_entity,
                            4.5,
                            timeline.spear_rotation,
                            energy_ttl,
                        );
                    }
                }
            }

            // Phase 2: Spike attack (lifeInt 8..14)
            if (DEATH_STAB_SPIKE_WINDOW_MIN..=DEATH_STAB_SPIKE_WINDOW_MAX).contains(&current_life)
                && rand_fps_check(2, factor, &mut rng)
            {
                if let Some(spear_path) = spear_glb {
                    let frame_into_spike =
                        (DEATH_STAB_SPIKE_WINDOW_MAX - current_life).max(0) as f32;
                    let distance = DEATH_STAB_SPIKE_BASE_DISTANCE + frame_into_spike * 10.0;
                    let spike_pos = caster_pos + timeline.forward_xz * distance + Vec3::Y * 120.0;
                    for _ in 0..DEATH_STAB_SPIKES_PER_SPAWN {
                        spawn_spike_particle(
                            &mut commands,
                            &asset_server,
                            spear_path,
                            spike_pos,
                            1.2,
                            timeline.spear_rotation,
                            spike_ttl,
                        );
                    }
                }
            }

            // Phase 3: Victim lightning at lifeInt 10
            if current_life == DEATH_STAB_IMPACT_LIFE_INT && !timeline.impact_applied {
                timeline.impact_applied = true;
                let center = target_pos + Vec3::Y * 80.0;
                commands.spawn((
                    RuntimeSceneEntity,
                    LightningHurtEffect {
                        remaining_frames: DEATH_STAB_HURT_FRAMES,
                        frame_accumulator: 0.0,
                        target_entity: timeline.target_entity,
                        fallback_center: center,
                        next_arc_seq: 0,
                        cached_bones: Vec::new(),
                        initialized: false,
                    },
                    Transform::from_translation(center),
                    GlobalTransform::default(),
                ));
            }
        }

        // Advance timeline
        timeline.life_frames -= factor;
        if timeline.life_frames <= 0.0 {
            commands.entity(timeline.charge_light_entity).try_despawn();
            commands.entity(timeline.impact_light_entity).try_despawn();
            commands.entity(entity).despawn();
        }
    }
}

// ============================================================================
// Material override system (replicates apply_skill_vfx_materials from
// character_viewer.rs line 5605)
// ============================================================================

/// Apply additive/unlit/emissive material overrides to DeathStab VFX particles
/// once their GLB scene has loaded and materials are available.
pub fn apply_death_stab_vfx_materials(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    vfx_roots: Query<
        Entity,
        (
            With<DeathStabVfxParticle>,
            Without<DeathStabMaterialsApplied>,
        ),
    >,
    children_query: Query<&Children>,
    material_query: Query<(Entity, &MeshMaterial3d<StandardMaterial>)>,
    material_presence: Query<(), With<MeshMaterial3d<StandardMaterial>>>,
) {
    for root_entity in &vfx_roots {
        // Gate: only process once the GLB subtree contains loaded materials
        if !subtree_contains_material(root_entity, &children_query, &material_presence) {
            continue;
        }

        // Walk entire subtree BFS to find mesh entities with materials
        let mut queue = vec![root_entity];
        while let Some(entity) = queue.pop() {
            if let Ok((mesh_entity, mat_handle)) = material_query.get(entity) {
                if let Some(original) = materials.get(&mat_handle.0).cloned() {
                    let mut overridden = original;
                    overridden.alpha_mode = AlphaMode::Add;
                    overridden.unlit = true;
                    // base_color modulates the texture; WHITE = full brightness initially,
                    // per-frame update systems modulate this for fade-in/fade-out.
                    // With unlit=true, shader output = base_color * base_color_texture.
                    // Black-background textures work: black pixels add nothing (additive).
                    overridden.base_color = Color::WHITE;
                    overridden.double_sided = true;
                    overridden.cull_mode = None;

                    let new_handle = materials.add(overridden);
                    commands
                        .entity(mesh_entity)
                        .insert(MeshMaterial3d(new_handle))
                        .insert(NotShadowCaster)
                        .insert(NotShadowReceiver);
                }
            }
            if let Ok(children) = children_query.get(entity) {
                queue.extend(children.iter());
            }
        }

        commands
            .entity(root_entity)
            .insert(DeathStabMaterialsApplied)
            .insert(Visibility::Inherited);
    }
}

// ============================================================================
// Energy particle update system (lerp position + emissive fade-in)
// ============================================================================

/// Lerp energy particles from start_pos to target_pos and fade-in via emissive.
/// Replicates C# `Vector3.Lerp(startPos, targetPos, progress)` and
/// `BlendMeshLight = (1 - progress) * 2`.
pub fn update_death_stab_energy_particles(
    time: Res<Time>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    global_transforms: Query<&GlobalTransform>,
    mut particles: Query<
        (Entity, &mut DeathStabEnergyParticle, &mut Transform),
        With<DeathStabMaterialsApplied>,
    >,
    children_query: Query<&Children>,
    material_query: Query<&MeshMaterial3d<StandardMaterial>>,
) {
    let dt = time.delta_secs();

    for (entity, mut particle, mut transform) in &mut particles {
        particle.elapsed_secs += dt;
        let progress = (particle.elapsed_secs / particle.max_lifetime_secs).clamp(0.0, 1.0);
        let eased_progress = progress * progress;
        let target_pos = particle
            .target_entity
            .and_then(|target| global_transforms.get(target).ok())
            .map(|gt| gt.translation())
            .unwrap_or(particle.target_pos);

        // Lerp position with ease-in for stronger "pull" toward the weapon tip near impact.
        transform.translation = particle.start_pos.lerp(target_pos, eased_progress);

        // C# BlendMeshLight = (1 - progress) * 2
        // Allow HDR overbright (up to 2.0) — no clamping to 1.0.
        // Note: C# Alpha=0.1 does NOT affect D3D additive blending (src+dst ignores alpha),
        // so we don't apply it here. BlendMeshLight is the sole brightness control.
        let brightness = ((1.0 - progress) * 2.0).max(0.0);
        let color = Color::srgb(brightness * 0.6, brightness * 0.8, brightness * 1.0);
        apply_emissive_recursive(
            entity,
            color,
            &children_query,
            &material_query,
            &mut materials,
        );
    }
}

// ============================================================================
// Spike particle update system (emissive fade-out)
// ============================================================================

/// Fade out spike particles via emissive intensity.
/// Replicates C# `BlendMeshLight = (lifeFrames / 10) * 1.5`.
pub fn update_death_stab_spike_particles(
    time: Res<Time>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut particles: Query<(Entity, &mut DeathStabSpikeParticle), With<DeathStabMaterialsApplied>>,
    children_query: Query<&Children>,
    material_query: Query<&MeshMaterial3d<StandardMaterial>>,
) {
    let dt = time.delta_secs();

    for (root_entity, mut particle) in &mut particles {
        particle.elapsed_secs += dt;
        let remaining_ratio =
            1.0 - (particle.elapsed_secs / particle.max_lifetime_secs).clamp(0.0, 1.0);

        // Fade-out: C# BlendMeshLight = (lifeFrames / 10) * 1.5
        // Allow HDR overbright (up to 1.5 at spawn) — no clamping to 1.0
        let intensity = (remaining_ratio * 1.5).max(0.0);
        let color = Color::srgb(intensity, intensity, intensity);
        apply_emissive_recursive(
            root_entity,
            color,
            &children_query,
            &material_query,
            &mut materials,
        );
    }
}

// ============================================================================
// Animation player initialization (replicates
// ensure_skill_vfx_animation_players from character_viewer.rs line 5184)
// ============================================================================

/// Find AnimationPlayers in DeathStab VFX subtrees, create AnimationGraphs,
/// and start playback with the configured speed.
pub fn ensure_death_stab_animation_players(
    mut commands: Commands,
    gltfs: Res<Assets<Gltf>>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
    children_query: Query<&Children>,
    player_presence: Query<(), With<AnimationPlayer>>,
    mut players: Query<&mut AnimationPlayer>,
    sources: Query<(Entity, &DeathStabAnimationSource), Without<DeathStabAnimationInitialized>>,
) {
    for (source_entity, source) in &sources {
        let player_entities =
            find_animation_players_in_subtree(source_entity, &children_query, &player_presence);
        if player_entities.is_empty() {
            continue;
        }

        let Some(gltf) = gltfs.get(&source.gltf_handle) else {
            continue;
        };

        if gltf.animations.is_empty() {
            commands
                .entity(source_entity)
                .insert(DeathStabAnimationInitialized);
            continue;
        }

        let mut graph = AnimationGraph::new();
        let nodes: Vec<AnimationNodeIndex> = graph
            .add_clips(gltf.animations.iter().cloned(), 1.0, graph.root)
            .collect();
        let Some(first_node) = nodes.first().copied() else {
            commands
                .entity(source_entity)
                .insert(DeathStabAnimationInitialized);
            continue;
        };
        let graph_handle = graphs.add(graph);

        for player_entity in player_entities {
            if let Ok(mut player) = players.get_mut(player_entity) {
                let mut transitions = AnimationTransitions::new();
                transitions
                    .play(&mut player, first_node, Duration::ZERO)
                    .set_speed(source.playback_speed.max(0.001))
                    .repeat();
                commands
                    .entity(player_entity)
                    .insert((AnimationGraphHandle(graph_handle.clone()), transitions));
            }
        }

        commands
            .entity(source_entity)
            .insert(DeathStabAnimationInitialized);
    }
}

// ============================================================================
// Lightning hurt effect system
// ============================================================================

/// Collect all bone entities from the target's hierarchy once,
/// then mark the effect as initialized so update_lightning_hurt_effects
/// can use them without BFS every frame.
pub fn initialize_lightning_hurt_effects(
    mut commands: Commands,
    children_query: Query<&Children>,
    global_transforms: Query<&GlobalTransform>,
    mut effects: Query<
        (Entity, &mut LightningHurtEffect),
        Without<LightningHurtEffectInitialized>,
    >,
) {
    for (entity, mut effect) in &mut effects {
        if effect.initialized {
            continue;
        }

        let mut bones = Vec::new();
        if let Some(target) = effect.target_entity {
            // Collect ALL descendants with GlobalTransform (like C#: boneTransforms array)
            // No name filtering — C# uses random index over all bones
            let mut queue = vec![target];
            while let Some(e) = queue.pop() {
                if bones.len() >= 48 {
                    break;
                }
                if e != target && global_transforms.contains(e) {
                    bones.push(e);
                }
                if let Ok(children) = children_query.get(e) {
                    queue.extend(children.iter());
                }
            }
        }

        effect.cached_bones = bones;
        effect.initialized = true;
        commands
            .entity(entity)
            .insert(LightningHurtEffectInitialized);
    }
}

pub fn spawn_lightning_overlay_camera(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        Camera {
            order: 10,
            clear_color: ClearColorConfig::None,
            ..default()
        },
        RuntimeSceneEntity,
        LightningOverlayCamera,
    ));
}

pub fn update_lightning_hurt_effects(
    mut commands: Commands,
    time: Res<Time>,
    asset_server: Res<AssetServer>,
    global_transforms: Query<&GlobalTransform>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<LightningSprite2dMaterial>>,
    mut cached_assets: Local<DeathStabLightningAssets>,
    mut warned_missing_texture: Local<bool>,
    mut effects: Query<
        (Entity, &mut LightningHurtEffect, &mut Transform),
        With<LightningHurtEffectInitialized>,
    >,
    active_arcs: Query<(Entity, &DeathStabLightningArc), With<DeathStabLightningArcVisual>>,
) {
    let mesh_handle = cached_assets
        .mesh
        .get_or_insert_with(|| meshes.add(Mesh::from(Rectangle::new(1.0, 1.0))))
        .clone();
    let material_handle = if let Some(material) = cached_assets.material.as_ref().cloned() {
        Some(material)
    } else if let Some(texture_path) = lightning_texture_asset() {
        let texture: Handle<Image> = asset_server.load_with_settings(
            resolve_asset_path(texture_path),
            |settings: &mut _| {
                *settings = ImageLoaderSettings {
                    is_srgb: true,
                    sampler: ImageSampler::Descriptor(ImageSamplerDescriptor {
                        address_mode_u: ImageAddressMode::Repeat,
                        address_mode_v: ImageAddressMode::Repeat,
                        address_mode_w: ImageAddressMode::Repeat,
                        ..default()
                    }),
                    ..default()
                };
            },
        );
        let material = materials.add(LightningSprite2dMaterial {
            color_texture: Some(texture),
            params: LightningSprite2dParams {
                intensity: DEATH_STAB_LIGHTNING_INTENSITY,
                _padding: Vec3::ZERO,
            },
        });
        cached_assets.material = Some(material.clone());
        Some(material)
    } else {
        if !*warned_missing_texture {
            warn!(
                "DeathStab lightning texture is missing: expected data/effect/joint_thunder_01.png (or fallback thunder_01/eff_lighting)"
            );
            *warned_missing_texture = true;
        }
        None
    };
    let Some(material_handle) = material_handle else {
        return;
    };

    let dt = time.delta_secs();
    let factor = fps_animation_factor(dt);
    let mut rng = rand::thread_rng();

    for (entity, mut effect, mut transform) in &mut effects {
        // Early-exit: if target entity has despawned, remove effect immediately (like C# RemoveSelf)
        if let Some(target_entity) = effect.target_entity {
            if global_transforms.get(target_entity).is_err() {
                commands.entity(entity).try_despawn();
                continue;
            }
        }

        let center = if let Some(target_entity) = effect.target_entity {
            global_transforms
                .get(target_entity)
                .map(|gt| gt.translation() + Vec3::Y * 80.0)
                .unwrap_or(effect.fallback_center)
        } else {
            effect.fallback_center
        };
        transform.translation = center;

        effect.frame_accumulator += factor;
        while effect.frame_accumulator >= 1.0 && effect.remaining_frames > 0 {
            effect.frame_accumulator -= 1.0;
            effect.remaining_frames -= 1;
        }

        if effect.remaining_frames == 0 {
            commands.entity(entity).try_despawn();
            continue;
        }

        if !rand_fps_check(2, factor, &mut rng) {
            continue;
        }

        let life = effect.remaining_frames as f32 / DEATH_STAB_HURT_FRAMES as f32;
        let arc_count = (8.0 + 8.0 * life)
            .round()
            .clamp(1.0, DEATH_STAB_MAX_ACTIVE_LIGHTNING_ARCS as f32)
            as usize;
        let mut active_for_owner: Vec<(u32, Entity)> = active_arcs
            .iter()
            .filter(|(_, arc)| arc.owner_effect == entity)
            .map(|(arc_entity, arc)| (arc.spawn_seq, arc_entity))
            .collect();
        let overflow = active_for_owner
            .len()
            .saturating_add(arc_count)
            .saturating_sub(DEATH_STAB_MAX_ACTIVE_LIGHTNING_ARCS);
        if overflow > 0 {
            active_for_owner.sort_by_key(|(seq, _)| *seq);
            for (_, arc_entity) in active_for_owner.into_iter().take(overflow) {
                commands.entity(arc_entity).try_despawn();
            }
        }

        // Use cached bones (collected once by initialize_lightning_hurt_effects).
        // Like C#, we pick random bone indices for arc start/end positions.
        let use_cached = effect.cached_bones.len() >= 2;

        for _ in 0..arc_count {
            let (bone1_pos, bone2_pos) = if use_cached {
                let idx1 = rng.gen_range(0..effect.cached_bones.len());
                let mut idx2 = rng.gen_range(0..effect.cached_bones.len());
                if idx1 == idx2 {
                    idx2 = (idx2 + 1) % effect.cached_bones.len();
                }
                let s = global_transforms
                    .get(effect.cached_bones[idx1])
                    .map(|t| t.translation())
                    .unwrap_or(center);
                let e = global_transforms
                    .get(effect.cached_bones[idx2])
                    .map(|t| t.translation())
                    .unwrap_or(center);
                (s, e)
            } else {
                // Fallback when no bones available
                let fallback = fallback_lightning_anchor_points(center);
                let idx1 = rng.gen_range(0..fallback.len());
                let mut idx2 = rng.gen_range(0..fallback.len());
                if idx1 == idx2 {
                    idx2 = (idx2 + 1) % fallback.len();
                }
                (fallback[idx1], fallback[idx2])
            };

            let jitter = Vec3::new(
                rng.gen_range(-14.0..=14.0),
                rng.gen_range(-14.0..=14.0),
                rng.gen_range(-14.0..=14.0),
            );
            let bone1_pos = bone1_pos + jitter;
            let bone2_pos = bone2_pos - jitter;
            let spawn_seq = effect.next_arc_seq;
            effect.next_arc_seq = effect.next_arc_seq.wrapping_add(1);

            commands.spawn((
                RuntimeSceneEntity,
                DeathStabLightningArcVisual,
                DeathStabLightningArc {
                    owner_effect: entity,
                    bone1_pos,
                    bone2_pos,
                    remaining_secs: DEATH_STAB_LIGHTNING_FLASH_SECS,
                    spawn_seq,
                },
                Mesh2d(mesh_handle.clone()),
                MeshMaterial2d(material_handle.clone()),
                Transform::default(),
                Visibility::Hidden,
            ));
        }
    }
}

pub fn update_death_stab_lightning_arcs(
    mut commands: Commands,
    time: Res<Time>,
    camera_3d_query: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    windows: Query<&Window>,
    mut arcs: Query<
        (
            Entity,
            &mut DeathStabLightningArc,
            &mut Transform,
            &mut Visibility,
        ),
        With<DeathStabLightningArcVisual>,
    >,
) {
    let dt = time.delta_secs();
    let Ok((camera, camera_gt)) = camera_3d_query.single() else {
        return;
    };
    let Ok(window) = windows.single() else {
        return;
    };
    let half_w = window.width() / 2.0;
    let half_h = window.height() / 2.0;
    let camera_pos = camera_gt.translation();

    for (entity, mut arc, mut transform, mut visibility) in &mut arcs {
        arc.remaining_secs -= dt;
        if arc.remaining_secs <= 0.0 {
            commands.entity(entity).try_despawn();
            continue;
        }

        // C#: midPoint = (bone1 + bone2) * 0.5
        let midpoint = (arc.bone1_pos + arc.bone2_pos) * 0.5;

        // Project 3D -> viewport via Camera3d
        let Ok(viewport_pos) = camera.world_to_viewport(camera_gt, midpoint) else {
            *visibility = Visibility::Hidden;
            continue;
        };

        // C#: direction = bone2Pos - bone1Pos (3D, NOT projected)
        let direction = arc.bone2_pos - arc.bone1_pos;
        let bone_distance = direction.length();
        if bone_distance < 0.001 {
            *visibility = Visibility::Hidden;
            continue;
        }

        // C#: angleZ = atan2(direction.Y, direction.X) — uses 3D direction directly
        let angle_z = direction.y.atan2(direction.x);

        // C#: Scale = Max(boneDistance / 50, 0.5)
        let base_scale = (bone_distance / 50.0).max(0.5);

        // C#: _scaleMix = Scale * TERRAIN_SIZE / Max(distToCamera, 0.1)
        let dist_to_camera = camera_pos.distance(midpoint).max(0.1);
        let scale_mix = base_scale * TERRAIN_SIZE / dist_to_camera;

        // Convert viewport coords (0,0 top-left) to Camera2d coords (0,0 center, Y-up)
        let screen_x = viewport_pos.x - half_w;
        let screen_y = half_h - viewport_pos.y;

        transform.translation = Vec3::new(screen_x, screen_y, 0.0);
        transform.rotation = Quat::from_rotation_z(angle_z);
        // Uniform: scale texture 256x32 by scale_mix (like C# SpriteBatch.Draw)
        transform.scale = Vec3::new(
            LIGHTNING_TEX_WIDTH * scale_mix,
            LIGHTNING_TEX_HEIGHT * scale_mix,
            1.0,
        );
        *visibility = Visibility::Inherited;
    }
}

// ============================================================================
// Auto-lifetime system
// ============================================================================

/// Despawn entities whose `SkillVfxAutoLifetime` timer has expired.
pub fn update_skill_vfx_auto_lifetimes(
    mut commands: Commands,
    time: Res<Time>,
    mut lifetimes: Query<(Entity, &mut SkillVfxAutoLifetime)>,
) {
    for (entity, mut lifetime) in &mut lifetimes {
        lifetime.timer.tick(time.delta());
        if lifetime.timer.just_finished() {
            commands.entity(entity).try_despawn();
        }
    }
}
