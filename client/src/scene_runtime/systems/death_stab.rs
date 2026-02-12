use crate::bevy_compat::PbrBundle;
use crate::scene_runtime::components::*;
use bevy::gltf::Gltf;
use bevy::light::{NotShadowCaster, NotShadowReceiver};
use bevy::math::primitives::Rectangle;
use bevy::pbr::MeshMaterial3d;
use bevy::prelude::*;
use bevy::time::Timer;
use rand::Rng;
use std::time::Duration;

// ============================================================================
// Constants (from C# DeathStabEffect.cs timing model)
// ============================================================================

const DEATH_STAB_TOTAL_LIFE_FRAMES: f32 = 20.0;
const DEATH_STAB_REFERENCE_FPS: f32 = 25.0;
const DEATH_STAB_MAX_ANIMATION_FACTOR: f32 = 2.5;
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
const DEATH_STAB_CHARGE_LIGHT_INTENSITY: f32 = 4_500.0;
const DEATH_STAB_IMPACT_LIGHT_INTENSITY: f32 = 8_000.0;
const DEATH_STAB_LIGHTNING_FLASH_SECS: f32 = 0.06;
const DEATH_STAB_LIGHTNING_WIDTH_BASE: f32 = 20.0;
const DEATH_STAB_LIGHTNING_WIDTH_JITTER: f32 = 4.0;
const DEATH_STAB_LIGHTNING_INTENSITY: f32 = 2.8;
const DEATH_STAB_ENERGY_HDR_SCALE: f32 = 2.5;
const DEATH_STAB_SPIKE_HDR_SCALE: f32 = 2.0;

const CLIENT_ASSETS_ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../assets");

#[derive(Default)]
pub struct DeathStabLightningAssets {
    mesh: Option<Handle<Mesh>>,
    material: Option<Handle<StandardMaterial>>,
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
    let full = format!("{}/{}", CLIENT_ASSETS_ROOT, path);
    std::path::Path::new(&full).exists()
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
    // Tries Spear01 first, falls back to Spear02
    if vfx_asset_exists("data/item/spear_01.glb") {
        Some("data/item/spear_01.glb")
    } else if vfx_asset_exists("data/item/spear_02.glb") {
        Some("data/item/spear_02.glb")
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
    uniform_scale: f32,
    rotation: Quat,
    ttl_seconds: f32,
) -> Entity {
    let scene_handle: Handle<Scene> = asset_server.load(format!("{glb_path}#Scene0"));
    let gltf_handle: Handle<Gltf> = asset_server.load(glb_path.to_string());
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
    let scene_handle: Handle<Scene> = asset_server.load(format!("{glb_path}#Scene0"));
    let gltf_handle: Handle<Gltf> = asset_server.load(glb_path.to_string());
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
        let weapon_tip_pos =
            caster_pos + timeline.forward_xz * DEATH_STAB_ENERGY_TARGET_DISTANCE + Vec3::Y * 120.0;
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
            impact_transform.translation = timeline.target_pos + Vec3::Y * 80.0;
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
                    for _ in 0..3 {
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
                    for _ in 0..2 {
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
                let center = timeline.target_pos + Vec3::Y * 80.0;
                commands.spawn((
                    RuntimeSceneEntity,
                    LightningHurtEffect {
                        remaining_frames: DEATH_STAB_HURT_FRAMES,
                        frame_accumulator: 0.0,
                        target_entity: None,
                        fallback_center: center,
                        next_arc_seq: 0,
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
                    overridden.base_color = Color::WHITE.into();
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
    mut particles: Query<
        (Entity, &mut DeathStabEnergyParticle, &mut Transform),
        With<DeathStabMaterialsApplied>,
    >,
    children_query: Query<&Children>,
    material_query: Query<&MeshMaterial3d<StandardMaterial>>,
) {
    let dt = time.delta_secs();

    for (_entity, mut particle, mut transform) in &mut particles {
        particle.elapsed_secs += dt;
        let progress = (particle.elapsed_secs / particle.max_lifetime_secs).clamp(0.0, 1.0);

        // Lerp position: C# Vector3.Lerp(startPos, targetPos, progress)
        transform.translation = particle.start_pos.lerp(particle.target_pos, progress);

        // C# BlendMeshLight = (1 - progress) * 2
        // Allow HDR overbright (up to 2.0) — no clamping to 1.0.
        // Note: C# Alpha=0.1 does NOT affect D3D additive blending (src+dst ignores alpha),
        // so we don't apply it here. BlendMeshLight is the sole brightness control.
        let brightness = ((1.0 - progress) * 2.0 * DEATH_STAB_ENERGY_HDR_SCALE).max(0.0);

        // Walk subtree to update all materials
        let mut queue = vec![_entity];
        while let Some(entity) = queue.pop() {
            if let Ok(mat_handle) = material_query.get(entity) {
                if let Some(mat) = materials.get_mut(&mat_handle.0) {
                    mat.base_color = Color::linear_rgba(brightness, brightness, brightness, 1.0);
                }
            }
            if let Ok(children) = children_query.get(entity) {
                queue.extend(children.iter());
            }
        }
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
        let intensity = (remaining_ratio * 1.5 * DEATH_STAB_SPIKE_HDR_SCALE).max(0.0);

        // Walk subtree to update all materials — modulate base_color directly.
        // With unlit=true: shader output = base_color * base_color_texture (additive blend).
        let mut queue = vec![root_entity];
        while let Some(entity) = queue.pop() {
            if let Ok(mat_handle) = material_query.get(entity) {
                if let Some(mat) = materials.get_mut(&mat_handle.0) {
                    mat.base_color = Color::linear_rgba(intensity, intensity, intensity, 1.0);
                }
            }
            if let Ok(children) = children_query.get(entity) {
                queue.extend(children.iter());
            }
        }
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

pub fn update_lightning_hurt_effects(
    mut commands: Commands,
    time: Res<Time>,
    asset_server: Res<AssetServer>,
    global_transforms: Query<&GlobalTransform>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut cached_assets: Local<DeathStabLightningAssets>,
    mut warned_missing_texture: Local<bool>,
    mut effects: Query<(Entity, &mut LightningHurtEffect, &mut Transform)>,
    active_arcs: Query<(Entity, &DeathStabLightningArc), With<DeathStabLightningArcVisual>>,
) {
    let mesh_handle = cached_assets
        .mesh
        .get_or_insert_with(|| meshes.add(Mesh::from(Rectangle::new(1.0, 1.0))))
        .clone();
    let material_handle = if let Some(material) = cached_assets.material.as_ref().cloned() {
        Some(material)
    } else if let Some(texture_path) = lightning_texture_asset() {
        let texture: Handle<Image> = asset_server.load(texture_path.to_string());
        let material = materials.add(StandardMaterial {
            base_color_texture: Some(texture),
            base_color: Color::linear_rgba(
                0.75 * DEATH_STAB_LIGHTNING_INTENSITY,
                0.9 * DEATH_STAB_LIGHTNING_INTENSITY,
                1.0 * DEATH_STAB_LIGHTNING_INTENSITY,
                1.0,
            ),
            alpha_mode: AlphaMode::Add,
            unlit: true,
            double_sided: true,
            cull_mode: None,
            perceptual_roughness: 1.0,
            metallic: 0.0,
            reflectance: 0.0,
            ..default()
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
            commands.entity(entity).despawn();
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
                commands.entity(arc_entity).despawn();
            }
        }

        let pseudo_bones = [
            Vec3::new(0.0, 74.0, 0.0),
            Vec3::new(0.0, 58.0, 12.0),
            Vec3::new(0.0, 58.0, -12.0),
            Vec3::new(18.0, 52.0, 0.0),
            Vec3::new(-18.0, 52.0, 0.0),
            Vec3::new(28.0, 30.0, 8.0),
            Vec3::new(-28.0, 30.0, 8.0),
            Vec3::new(20.0, 6.0, 4.0),
            Vec3::new(-20.0, 6.0, 4.0),
            Vec3::new(0.0, 40.0, 18.0),
            Vec3::new(0.0, 40.0, -18.0),
            Vec3::new(0.0, 22.0, 0.0),
        ];

        for _ in 0..arc_count {
            let start_index = rng.gen_range(0..pseudo_bones.len());
            let mut end_index = rng.gen_range(0..pseudo_bones.len());
            if start_index == end_index {
                end_index = (end_index + 1) % pseudo_bones.len();
            }
            let start = center
                + pseudo_bones[start_index]
                + Vec3::new(
                    rng.gen_range(-20.0..=20.0),
                    rng.gen_range(-20.0..=20.0),
                    rng.gen_range(-20.0..=20.0),
                );
            let end = center
                + pseudo_bones[end_index]
                + Vec3::new(
                    rng.gen_range(-20.0..=20.0),
                    rng.gen_range(-20.0..=20.0),
                    rng.gen_range(-20.0..=20.0),
                );
            let width = (DEATH_STAB_LIGHTNING_WIDTH_BASE
                + rng.gen_range(
                    -DEATH_STAB_LIGHTNING_WIDTH_JITTER..=DEATH_STAB_LIGHTNING_WIDTH_JITTER,
                ))
            .max(4.0);
            let spawn_seq = effect.next_arc_seq;
            effect.next_arc_seq = effect.next_arc_seq.wrapping_add(1);

            commands.spawn((
                RuntimeSceneEntity,
                DeathStabLightningArcVisual,
                DeathStabLightningArc {
                    owner_effect: entity,
                    start,
                    end,
                    width,
                    remaining_secs: DEATH_STAB_LIGHTNING_FLASH_SECS,
                    spawn_seq,
                },
                NotShadowCaster,
                NotShadowReceiver,
                PbrBundle {
                    mesh: Mesh3d(mesh_handle.clone()),
                    material: MeshMaterial3d(material_handle.clone()),
                    transform: Transform::from_translation((start + end) * 0.5),
                    visibility: Visibility::Inherited,
                    ..default()
                },
            ));
        }
    }
}

pub fn update_death_stab_lightning_arcs(
    mut commands: Commands,
    time: Res<Time>,
    camera_query: Query<&GlobalTransform, With<Camera3d>>,
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
    let camera_position = camera_query
        .single()
        .ok()
        .map(|camera| camera.translation());

    for (entity, mut arc, mut transform, mut visibility) in &mut arcs {
        arc.remaining_secs -= dt;
        if arc.remaining_secs <= 0.0 {
            commands.entity(entity).despawn();
            continue;
        }

        let direction = arc.end - arc.start;
        let length = direction.length();
        if length <= 0.001 {
            *visibility = Visibility::Hidden;
            continue;
        }

        let Some(camera_position) = camera_position else {
            *visibility = Visibility::Hidden;
            continue;
        };

        let forward = direction / length;
        let center = (arc.start + arc.end) * 0.5;
        let to_camera = (camera_position - center).normalize_or_zero();

        let mut right = forward.cross(to_camera);
        if right.length_squared() <= 0.000_001 {
            let fallback = if forward.y.abs() < 0.99 {
                Vec3::Y
            } else {
                Vec3::X
            };
            right = forward.cross(fallback);
        }
        if right.length_squared() <= 0.000_001 {
            *visibility = Visibility::Hidden;
            continue;
        }
        right = right.normalize();

        let face = right.cross(forward).normalize_or_zero();
        if face.length_squared() <= 0.000_001 {
            *visibility = Visibility::Hidden;
            continue;
        }

        transform.translation = center;
        transform.rotation = Quat::from_mat3(&Mat3::from_cols(right, forward, face));
        transform.scale = Vec3::new(arc.width.max(1.0), length.max(0.001), 1.0);
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
