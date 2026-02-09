use crate::scene_runtime::components::{
    SceneObjectAnimationInitialized, SceneObjectAnimationSource,
};
use bevy::gltf::Gltf;
use bevy::prelude::*;
use std::collections::HashMap;
use std::time::Duration;

enum CachedSceneObjectAnimation {
    Ready {
        graph: Handle<AnimationGraph>,
        first_clip: AnimationNodeIndex,
    },
    NoAnimations,
}

#[derive(Default)]
pub(crate) struct SceneObjectAnimationCache {
    by_asset_path: HashMap<String, CachedSceneObjectAnimation>,
}

enum ResolvedSceneObjectAnimation {
    Ready {
        graph: Handle<AnimationGraph>,
        first_clip: AnimationNodeIndex,
    },
    NoAnimations,
}

#[derive(Default)]
pub(crate) struct AnimationDiagnostics {
    timer: Option<Timer>,
    pending_sources: u32,
    gltf_not_loaded: u32,
    awaiting_player: u32,
    animations_started: u32,
    no_animations: u32,
    total_started: u32,
    first_success_logged: bool,
}

/// Ensure scene objects with animations get their AnimationPlayers initialized.
///
/// Takes a **source-to-player** approach: iterates over entities that have
/// `SceneObjectAnimationSource` but not yet `SceneObjectAnimationInitialized`, searches
/// their children subtree for ALL `AnimationPlayer` entities (auto-spawned by Bevy's GLTF
/// scene loader), and initializes each with the animation graph and transitions.
pub fn ensure_scene_object_animation_players(
    mut commands: Commands,
    gltfs: Res<Assets<Gltf>>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
    children_query: Query<&Children>,
    mut players: Query<&mut AnimationPlayer>,
    sources: Query<
        (Entity, &SceneObjectAnimationSource),
        Without<SceneObjectAnimationInitialized>,
    >,
    mut cache: Local<SceneObjectAnimationCache>,
    mut diagnostics: Local<AnimationDiagnostics>,
    time: Res<Time>,
) {
    if diagnostics.timer.is_none() {
        diagnostics.timer = Some(Timer::from_seconds(3.0, TimerMode::Repeating));
    }
    let timer = diagnostics.timer.as_mut().unwrap();
    timer.tick(time.delta());
    let should_log = timer.just_finished();

    if should_log {
        diagnostics.pending_sources = 0;
        diagnostics.gltf_not_loaded = 0;
        diagnostics.awaiting_player = 0;
        diagnostics.animations_started = 0;
        diagnostics.no_animations = 0;
    }

    for (source_entity, source) in &sources {
        if should_log {
            diagnostics.pending_sources += 1;
        }

        let Some(resolved) = resolve_scene_object_animation(
            source,
            &gltfs,
            &mut graphs,
            &mut cache,
        ) else {
            if should_log {
                diagnostics.gltf_not_loaded += 1;
            }
            continue;
        };

        match resolved {
            ResolvedSceneObjectAnimation::NoAnimations => {
                if should_log {
                    diagnostics.no_animations += 1;
                }
                commands
                    .entity(source_entity)
                    .insert(SceneObjectAnimationInitialized);
            }
            ResolvedSceneObjectAnimation::Ready { graph, first_clip } => {
                // Find ALL AnimationPlayers in the subtree (GLBs with multiple root
                // nodes can have multiple AnimationPlayers — each must be initialized).
                let player_entities = find_all_animation_players_in_subtree(
                    source_entity,
                    &children_query,
                    &players,
                );

                if player_entities.is_empty() {
                    // Scene not fully instantiated yet. Retry next frame.
                    if should_log {
                        diagnostics.awaiting_player += 1;
                    }
                    continue;
                }

                // Initialize every AnimationPlayer found.
                for player_entity in &player_entities {
                    if let Ok(mut player) = players.get_mut(*player_entity) {
                        let mut transitions = AnimationTransitions::new();
                        transitions
                            .play(&mut player, first_clip, Duration::ZERO)
                            .set_speed(source.playback_speed.max(0.001))
                            .repeat();

                        commands.entity(*player_entity).insert((
                            graph.clone(),
                            transitions,
                        ));
                    }
                }

                commands
                    .entity(source_entity)
                    .insert(SceneObjectAnimationInitialized);

                if should_log {
                    diagnostics.animations_started += 1;
                }
                diagnostics.total_started += 1;
                if !diagnostics.first_success_logged {
                    diagnostics.first_success_logged = true;
                    info!(
                        "First scene object animation started: '{}' (speed={:.2}, players={})",
                        source.glb_asset_path, source.playback_speed, player_entities.len()
                    );
                }
            }
        }
    }

    if should_log
        && (diagnostics.pending_sources > 0
            || diagnostics.animations_started > 0)
    {
        info!(
            "Animation diagnostics: pending={}, started={}, no_anim={}, gltf_loading={}, awaiting_player={}, total_started={}",
            diagnostics.pending_sources,
            diagnostics.animations_started,
            diagnostics.no_animations,
            diagnostics.gltf_not_loaded,
            diagnostics.awaiting_player,
            diagnostics.total_started,
        );
    }
}

/// BFS through the entity subtree to find ALL entities with AnimationPlayer.
fn find_all_animation_players_in_subtree(
    root: Entity,
    children_query: &Query<&Children>,
    players: &Query<&mut AnimationPlayer>,
) -> Vec<Entity> {
    let mut result = Vec::new();
    let mut queue = vec![root];
    while let Some(entity) = queue.pop() {
        if players.contains(entity) {
            result.push(entity);
        }
        if let Ok(children) = children_query.get(entity) {
            queue.extend(children.iter());
        }
    }
    result
}

fn resolve_scene_object_animation(
    source: &SceneObjectAnimationSource,
    gltfs: &Assets<Gltf>,
    graphs: &mut Assets<AnimationGraph>,
    cache: &mut SceneObjectAnimationCache,
) -> Option<ResolvedSceneObjectAnimation> {
    if let Some(cached) = cache.by_asset_path.get(&source.glb_asset_path) {
        return Some(match cached {
            CachedSceneObjectAnimation::Ready { graph, first_clip } => {
                ResolvedSceneObjectAnimation::Ready {
                    graph: graph.clone(),
                    first_clip: *first_clip,
                }
            }
            CachedSceneObjectAnimation::NoAnimations => ResolvedSceneObjectAnimation::NoAnimations,
        });
    }

    let gltf = gltfs.get(&source.gltf_handle)?;

    if gltf.animations.is_empty() {
        cache.by_asset_path.insert(
            source.glb_asset_path.clone(),
            CachedSceneObjectAnimation::NoAnimations,
        );
        return Some(ResolvedSceneObjectAnimation::NoAnimations);
    }

    let mut graph = AnimationGraph::new();
    let clip_nodes: Vec<AnimationNodeIndex> = graph
        .add_clips(gltf.animations.iter().cloned(), 1.0, graph.root)
        .collect();
    let Some(first_clip) = clip_nodes.first().copied() else {
        cache.by_asset_path.insert(
            source.glb_asset_path.clone(),
            CachedSceneObjectAnimation::NoAnimations,
        );
        return Some(ResolvedSceneObjectAnimation::NoAnimations);
    };

    let graph_handle = graphs.add(graph);
    cache.by_asset_path.insert(
        source.glb_asset_path.clone(),
        CachedSceneObjectAnimation::Ready {
            graph: graph_handle.clone(),
            first_clip,
        },
    );

    Some(ResolvedSceneObjectAnimation::Ready {
        graph: graph_handle,
        first_clip,
    })
}
