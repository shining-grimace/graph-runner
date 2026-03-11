use std::time::Duration;

use bevy::prelude::*;

use crate::{
    controller::{Attachment, PlayerController, SpecialMove},
    loading::GameAssets,
};

const ANIMATION_INDEX_IDLE: usize = 0;
const ANIMATION_INDEX_WALKING: usize = 1;

#[derive(Resource)]
struct CharacterAnimations {
    animations: Vec<AnimationNodeIndex>,
}

pub struct AnimationPlugin;

impl Plugin for AnimationPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(set_up_character_graph)
            .add_observer(change_animation);
    }
}

fn set_up_character_graph(
    trigger: On<Add, AnimationPlayer>,
    mut commands: Commands,
    game_assets: Res<GameAssets>,
    player_query: Query<Entity, With<PlayerController>>,
    mut animation_players: Query<(Entity, &mut AnimationPlayer)>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
) -> Result<(), BevyError> {
    let Ok(character_entity) = player_query.single() else {
        return Err("There is no character to attach the animation to.".into());
    };
    if trigger.entity != character_entity {
        return Err("There is an animation player not targeted to the player.".into());
    }

    let clips = game_assets
        .character_animations
        .iter()
        .map(|handle| handle.clone());
    let (graph, node_indices) = AnimationGraph::from_clips(clips);
    let graph_handle = graphs.add(graph);
    let first_node = node_indices[0];

    commands.insert_resource(CharacterAnimations {
        animations: node_indices,
    });

    for (player_entity, mut animation_player) in &mut animation_players {
        if player_entity != trigger.entity {
            continue;
        }
        let mut transitions = AnimationTransitions::new();
        transitions
            .play(&mut animation_player, first_node, Duration::ZERO)
            .repeat();
        commands
            .entity(trigger.entity)
            .insert(AnimationGraphHandle(graph_handle.clone()))
            .insert(transitions);
    }

    Ok(())
}

fn change_animation(
    _: On<Add, SpecialMove>, // THIS EVENT ISN'T SUFFICIENT TO UPDATE ALL CASES!!!
    //WHAT I'M CONFUSED ON: DOES THE AnimationPlayer NEED TO BE ON THE SAME ENTITY AS THE ROOT BONE? OR THE MESH? OR THE PARENT ARMATURE?
    player_query: Query<
        (&ChildOf, Option<&Attachment>, Option<&SpecialMove>),
        With<PlayerController>,
    >,
    mut animation_players: Query<(Entity, &mut AnimationPlayer, &mut AnimationTransitions)>,
    animations: Res<CharacterAnimations>,
) -> Result<(), BevyError> {
    let Ok((character_parent, attachment, special_move)) = player_query.single() else {
        return Err("No character found to update the animation for.".into());
    };
    for (player_entity, mut animation_player, mut transitions) in &mut animation_players {
        if player_entity != character_parent.0 {
            continue;
        }
        let animation_index = player_animation_index_for(attachment, special_move);
        transitions
            .play(
                &mut animation_player,
                animations.animations[animation_index],
                Duration::from_millis(250),
            )
            .repeat();
    }
    Ok(())
}

fn player_animation_index_for(
    attachment: Option<&Attachment>,
    special_move: Option<&SpecialMove>,
) -> usize {
    match (attachment, special_move) {
        (Some(Attachment::Grounded { .. }), Some(SpecialMove::Running)) => ANIMATION_INDEX_WALKING,
        _ => ANIMATION_INDEX_IDLE,
    }
}
