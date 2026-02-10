use super::controller::CharacterAnimState;
use super::types::{BodyPartMarker, CharacterRoot};
use bevy::asset::AssetId;
use bevy::gltf::Gltf;
use bevy::prelude::*;
use std::collections::HashMap;
use std::time::Duration;

/// Shared animation library built from player.glb.
#[derive(Resource)]
pub struct PlayerAnimationLibrary {
    pub gltf_handle: Handle<Gltf>,
    pub graph_handle: Option<Handle<AnimationGraph>>,
    pub animation_nodes: Vec<AnimationNodeIndex>,
    pub animation_names: Vec<String>,
    pub initialized: bool,
}

impl PlayerAnimationLibrary {
    pub fn new(gltf_handle: Handle<Gltf>) -> Self {
        Self {
            gltf_handle,
            graph_handle: None,
            animation_nodes: Vec::new(),
            animation_names: Vec::new(),
            initialized: false,
        }
    }
}

/// Build the animation graph once the player.glb Gltf asset is loaded.
pub fn initialize_player_animation_library(
    mut library: ResMut<PlayerAnimationLibrary>,
    gltfs: Res<Assets<Gltf>>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
) {
    if library.initialized {
        return;
    }

    let Some(gltf) = gltfs.get(&library.gltf_handle) else {
        return;
    };

    library.initialized = true;

    if gltf.animations.is_empty() {
        info!("player.glb loaded but has no animations.");
        return;
    }

    let mut graph = AnimationGraph::new();
    let animation_nodes: Vec<AnimationNodeIndex> = graph
        .add_clips(gltf.animations.iter().cloned(), 1.0, graph.root)
        .collect();

    let mut names = Vec::with_capacity(gltf.animations.len());
    names.extend((0..gltf.animations.len()).map(|i| format!("Action{i:03}")));

    let index_by_clip_id: HashMap<AssetId<AnimationClip>, usize> = gltf
        .animations
        .iter()
        .enumerate()
        .map(|(i, h)| (h.id(), i))
        .collect();

    for (name, handle) in &gltf.named_animations {
        if let Some(&idx) = index_by_clip_id.get(&handle.id()) {
            names[idx] = name.to_string();
        }
    }

    info!(
        "PlayerAnimationLibrary: {} animation(s) loaded from player.glb",
        animation_nodes.len()
    );

    library.graph_handle = Some(graphs.add(graph));
    library.animation_nodes = animation_nodes;
    library.animation_names = names;
}

/// Marker for animation players already bound to the character animation system.
#[derive(Component)]
pub struct CharacterAnimationBound;

/// Detect new AnimationPlayer entities spawned under BodyPartMarker entities
/// and attach the shared animation graph + start the character's current animation.
pub fn bind_character_animation_players(
    mut commands: Commands,
    library: Res<PlayerAnimationLibrary>,
    character_query: Query<&CharacterAnimState, With<CharacterRoot>>,
    body_parts: Query<(Entity, &ChildOf), With<BodyPartMarker>>,
    children_query: Query<&Children>,
    mut players: Query<(Entity, &mut AnimationPlayer), Without<CharacterAnimationBound>>,
) {
    let Some(graph_handle) = library.graph_handle.clone() else {
        return;
    };

    for (part_entity, parent) in &body_parts {
        let Ok(anim_state) = character_query.get(parent.parent()) else {
            continue;
        };

        let animation_node = match library.animation_nodes.get(anim_state.current_action) {
            Some(&node) => node,
            None => {
                if let Some(&node) = library.animation_nodes.first() {
                    node
                } else {
                    continue;
                }
            }
        };

        // BFS to find all AnimationPlayers in the body part subtree
        let player_entities =
            find_animation_players_in_subtree(part_entity, &children_query, &players);

        for player_entity in player_entities {
            if let Ok((entity, mut player)) = players.get_mut(player_entity) {
                let mut transitions = AnimationTransitions::new();
                transitions
                    .play(&mut player, animation_node, Duration::ZERO)
                    .set_speed(anim_state.playback_speed.max(0.001))
                    .repeat();

                commands.entity(entity).insert((
                    AnimationGraphHandle(graph_handle.clone()),
                    transitions,
                    CharacterAnimationBound,
                ));
            }
        }
    }
}

/// When CharacterAnimState changes, transition all bound animation players.
pub fn apply_character_animation_changes(
    library: Res<PlayerAnimationLibrary>,
    changed_characters: Query<
        (Entity, &CharacterAnimState),
        (With<CharacterRoot>, Changed<CharacterAnimState>),
    >,
    children_query: Query<&Children>,
    mut players: Query<
        (&mut AnimationPlayer, &mut AnimationTransitions),
        With<CharacterAnimationBound>,
    >,
) {
    if library.animation_nodes.is_empty() {
        return;
    }

    for (root_entity, anim_state) in &changed_characters {
        let animation_node = match library.animation_nodes.get(anim_state.current_action) {
            Some(&node) => node,
            None => {
                if let Some(&node) = library.animation_nodes.first() {
                    node
                } else {
                    continue;
                }
            }
        };

        // Find all animation players in the entire character subtree
        let player_entities = find_bound_players_in_subtree(root_entity, &children_query, &players);

        for player_entity in player_entities {
            if let Ok((mut player, mut transitions)) = players.get_mut(player_entity) {
                transitions
                    .play(&mut player, animation_node, Duration::from_millis(200))
                    .set_speed(anim_state.playback_speed.max(0.001))
                    .repeat();
            }
        }
    }
}

fn find_animation_players_in_subtree(
    root: Entity,
    children_query: &Query<&Children>,
    players: &Query<(Entity, &mut AnimationPlayer), Without<CharacterAnimationBound>>,
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

fn find_bound_players_in_subtree(
    root: Entity,
    children_query: &Query<&Children>,
    players: &Query<
        (&mut AnimationPlayer, &mut AnimationTransitions),
        With<CharacterAnimationBound>,
    >,
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
