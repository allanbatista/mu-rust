use crate::scene_runtime::components::*;
use bevy::gltf::Gltf;
use bevy::light::{NotShadowCaster, NotShadowReceiver};
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

const CLIENT_ASSETS_ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../assets");

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
            if (DEATH_STAB_ENERGY_WINDOW_MIN..=DEATH_STAB_ENERGY_WINDOW_MAX)
                .contains(&current_life)
                && rand_fps_check(1, factor, &mut rng)
            {
                if let Some(charge_path) = charge_glb {
                    for _ in 0..3 {
                        // C#: Random.Next(-300, 300) independently for X and Y (rectangular)
                        let spread = Vec3::new(
                            rng.gen_range(-DEATH_STAB_ENERGY_RANDOM_RADIUS..DEATH_STAB_ENERGY_RANDOM_RADIUS),
                            0.0,
                            rng.gen_range(-DEATH_STAB_ENERGY_RANDOM_RADIUS..DEATH_STAB_ENERGY_RANDOM_RADIUS),
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
            if (DEATH_STAB_SPIKE_WINDOW_MIN..=DEATH_STAB_SPIKE_WINDOW_MAX)
                .contains(&current_life)
                && rand_fps_check(2, factor, &mut rng)
            {
                if let Some(spear_path) = spear_glb {
                    let frame_into_spike =
                        (DEATH_STAB_SPIKE_WINDOW_MAX - current_life).max(0) as f32;
                    let distance = DEATH_STAB_SPIKE_BASE_DISTANCE + frame_into_spike * 10.0;
                    let spike_pos =
                        caster_pos + timeline.forward_xz * distance + Vec3::Y * 120.0;
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
                    overridden.unlit = false; // Must be false for emissive to work in Bevy 0.18 PBR pipeline
                    overridden.base_color = Color::BLACK.into(); // Zero out lit channel so only emissive contributes
                    overridden.double_sided = true;
                    overridden.cull_mode = None;
                    overridden.emissive = LinearRgba::WHITE;
                    overridden.emissive_exposure_weight = 0.0; // Prevent camera exposure from scaling emissive
                    if overridden.emissive_texture.is_none() {
                        overridden.emissive_texture = overridden.base_color_texture.clone();
                    }
                    overridden.perceptual_roughness = 1.0;
                    overridden.metallic = 0.0;
                    overridden.reflectance = 0.0;

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

        // Emissive fade-in: C# BlendMeshLight = (1 - progress) * 2
        // Allow HDR overbright (up to 2.0) — no clamping to 1.0
        let intensity = ((1.0 - progress) * 2.0).max(0.0);

        // C# Alpha = 0.1f — each energy particle is ghostly-dim; many overlap additively
        // With AlphaMode::Add, alpha doesn't modulate color, so bake it into emissive RGB
        let alpha = 0.1;
        let emissive_value = intensity * alpha;

        // Walk subtree to update all materials
        let mut queue = vec![_entity];
        while let Some(entity) = queue.pop() {
            if let Ok(mat_handle) = material_query.get(entity) {
                if let Some(mat) = materials.get_mut(&mat_handle.0) {
                    mat.emissive = LinearRgba::new(emissive_value, emissive_value, emissive_value, 1.0);
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
    mut particles: Query<
        (Entity, &mut DeathStabSpikeParticle),
        With<DeathStabMaterialsApplied>,
    >,
    children_query: Query<&Children>,
    material_query: Query<&MeshMaterial3d<StandardMaterial>>,
) {
    let dt = time.delta_secs();

    for (root_entity, mut particle) in &mut particles {
        particle.elapsed_secs += dt;
        let remaining_ratio = 1.0 - (particle.elapsed_secs / particle.max_lifetime_secs).clamp(0.0, 1.0);

        // Emissive fade-out: C# BlendMeshLight = (lifeFrames / 10) * 1.5
        // Allow HDR overbright (up to 1.5 at spawn) — no clamping to 1.0
        let intensity = (remaining_ratio * 1.5).max(0.0);

        // Walk subtree to update all materials
        let mut queue = vec![root_entity];
        while let Some(entity) = queue.pop() {
            if let Ok(mat_handle) = material_query.get(entity) {
                if let Some(mat) = materials.get_mut(&mat_handle.0) {
                    mat.emissive = LinearRgba::new(intensity, intensity, intensity, 1.0);
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
    sources: Query<
        (Entity, &DeathStabAnimationSource),
        Without<DeathStabAnimationInitialized>,
    >,
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
    mut gizmos: Gizmos,
    global_transforms: Query<&GlobalTransform>,
    mut effects: Query<(Entity, &mut LightningHurtEffect, &mut Transform)>,
) {
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
            .clamp(1.0, DEATH_STAB_MAX_ACTIVE_LIGHTNING_ARCS as f32) as usize;
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
            let color = Color::srgba(
                rng.gen_range(0.65..=0.95),
                rng.gen_range(0.75..=0.95),
                1.0,
                (0.25 + life * 0.70).clamp(0.0, 1.0),
            );
            gizmos.line(start, end, color);
        }
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
