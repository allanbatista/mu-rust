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

/// Start scene-object GLB animations when animation players become available.
///
/// This is resilient to async scene loading: players are retried until the
/// associated `Gltf` metadata is loaded and a graph can be built.
pub fn start_scene_object_animations(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    gltfs: Res<Assets<Gltf>>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
    parents: Query<&Parent>,
    scene_object_animation_sources: Query<&SceneObjectAnimationSource>,
    mut players: Query<(Entity, &mut AnimationPlayer), Without<SceneObjectAnimationInitialized>>,
    mut cache: Local<SceneObjectAnimationCache>,
) {
    for (player_entity, mut player) in &mut players {
        let Some(owner_entity) = find_ancestor_with_animation_source(
            player_entity,
            &parents,
            &scene_object_animation_sources,
        ) else {
            // Scene hierarchy can be attached asynchronously; retry on next frames.
            continue;
        };

        let Ok(source) = scene_object_animation_sources.get(owner_entity) else {
            continue;
        };

        let Some(resolved) =
            resolve_scene_object_animation(source, &asset_server, &gltfs, &mut graphs, &mut cache)
        else {
            // Associated Gltf asset is still loading.
            continue;
        };

        match resolved {
            ResolvedSceneObjectAnimation::Ready { graph, first_clip } => {
                let mut transitions = AnimationTransitions::new();
                transitions
                    .play(&mut player, first_clip, Duration::ZERO)
                    .set_speed(source.playback_speed.max(0.001))
                    .repeat();

                commands.entity(player_entity).insert((
                    graph,
                    transitions,
                    SceneObjectAnimationInitialized,
                ));
            }
            ResolvedSceneObjectAnimation::NoAnimations => {
                commands
                    .entity(player_entity)
                    .insert(SceneObjectAnimationInitialized);
            }
        }
    }
}

fn find_ancestor_with_animation_source(
    start: Entity,
    parents: &Query<&Parent>,
    animation_sources: &Query<&SceneObjectAnimationSource>,
) -> Option<Entity> {
    let mut current = start;

    loop {
        if animation_sources.get(current).is_ok() {
            return Some(current);
        }

        let Ok(parent) = parents.get(current) else {
            return None;
        };
        current = parent.get();
    }
}

fn resolve_scene_object_animation(
    source: &SceneObjectAnimationSource,
    asset_server: &AssetServer,
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

    let gltf_handle: Handle<Gltf> = asset_server.load(source.glb_asset_path.clone());
    let gltf = gltfs.get(&gltf_handle)?;

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
