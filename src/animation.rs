use std::time::Duration;

use bevy::prelude::*;

use crate::{
    InputSystems,
    controller::{Attachment, PlayerController, SpecialMove},
    loading::GameAssets,
    state::AppState,
};

const ANIMATION_INDEX_IDLE: usize = 0;
const ANIMATION_INDEX_WALKING: usize = 1;

#[derive(Resource)]
struct CharacterAnimations {
    current_index: Option<usize>,
    animations: Vec<AnimationNodeIndex>,
}

pub struct AnimationPlugin;

impl Plugin for AnimationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PreUpdate,
            update_player_animation
                .in_set(InputSystems::AfterStateUpdates)
                .run_if(in_state(AppState::Game)),
        )
        .add_observer(set_up_character_graph);
    }
}

fn set_up_character_graph(
    trigger: On<Add, AnimationPlayer>,
    mut commands: Commands,
    game_assets: Res<GameAssets>,
    mut player_query: Query<(Entity, &mut AnimationPlayer), With<PlayerController>>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
) -> Result<(), BevyError> {
    let Ok((player_entity, mut animation_player)) = player_query.single_mut() else {
        return Err("There is no character to attach the animation to.".into());
    };
    if player_entity != trigger.entity {
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
        current_index: None,
        animations: node_indices,
    });

    let mut transitions = AnimationTransitions::new();
    transitions
        .play(&mut animation_player, first_node, Duration::ZERO)
        .repeat();
    commands
        .entity(trigger.entity)
        .insert(AnimationGraphHandle(graph_handle.clone()))
        .insert(transitions);

    Ok(())
}

fn update_player_animation(
    mut player_query: Query<
        (
            Option<&Attachment>,
            Option<&SpecialMove>,
            &mut AnimationPlayer,
            &mut AnimationTransitions,
        ),
        With<PlayerController>,
    >,
    mut animations: ResMut<CharacterAnimations>,
) -> Result<(), BevyError> {
    let Ok((attachment, special_move, mut animation_player, mut transitions)) =
        player_query.single_mut()
    else {
        return Err("No character found to update the animation for.".into());
    };
    let applicable_animation_index = player_animation_index_for(attachment, special_move);
    if let Some(current_index) = animations.current_index {
        if current_index == applicable_animation_index {
            return Ok(());
        }
    }
    transitions
        .play(
            &mut animation_player,
            animations.animations[applicable_animation_index],
            Duration::from_millis(250),
        )
        .repeat();
    animations.current_index = Some(applicable_animation_index);
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
