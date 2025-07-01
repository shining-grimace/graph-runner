use crate::{markers::Player, state::AppState};
use avian3d::{prelude::*, schedule::PhysicsSet};
use bevy::prelude::*;

const JUMP_VELOCITY: f32 = 8.0;
const MOVE_ACCELERATION: f32 = 10.0;
const MOVE_MAX_SPEED: f32 = 8.0;

#[derive(Default, Resource)]
pub struct MovementState {
    input_direction_x: f32,
    pressing_jump: bool,
    just_pressed_jump: bool,
    has_contact: bool,
}

pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(MovementState::default())
            .add_systems(PreUpdate, poll_inputs.run_if(in_state(AppState::Game)))
            .add_systems(Update, move_player.run_if(in_state(AppState::Game)))
            .add_systems(
                PostUpdate,
                update_contacts
                    .run_if(in_state(AppState::Game))
                    .after(PhysicsSet::Sync),
            );
    }
}

fn poll_inputs(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut input_state: ResMut<MovementState>,
    mut exit_signal: EventWriter<AppExit>,
) {
    if keyboard_input.just_pressed(KeyCode::Escape) {
        exit_signal.write(AppExit::Success);
    }

    let pressing_left =
        keyboard_input.pressed(KeyCode::ArrowLeft) || keyboard_input.pressed(KeyCode::KeyA);
    let pressing_right =
        keyboard_input.pressed(KeyCode::ArrowRight) || keyboard_input.pressed(KeyCode::KeyD);
    input_state.input_direction_x = if pressing_left && pressing_right {
        0.0
    } else if pressing_left {
        -1.0
    } else if pressing_right {
        1.0
    } else {
        0.0
    };

    let was_previously_pressing_jump = input_state.pressing_jump;
    let now_pressing_jump =
        keyboard_input.pressed(KeyCode::Space) || keyboard_input.pressed(KeyCode::KeyL);
    input_state.pressing_jump = now_pressing_jump;
    input_state.just_pressed_jump = now_pressing_jump && !was_previously_pressing_jump;
}

fn move_player(
    mut player_query: Query<(&mut Transform, &mut LinearVelocity), With<Player>>,
    input_state: Res<MovementState>,
    time: Res<Time>,
) -> Result<(), BevyError> {
    let (mut player_transform, mut player_velocity) = player_query.single_mut()?;
    let time_delta = time.delta_secs();
    player_velocity.x = (player_velocity.x
        + input_state.input_direction_x * MOVE_ACCELERATION * time_delta)
        .min(MOVE_MAX_SPEED)
        .max(-MOVE_MAX_SPEED);
    player_velocity.z = 0.0;
    player_transform.translation.z = 0.0;
    if input_state.just_pressed_jump && input_state.has_contact {
        player_velocity.y = JUMP_VELOCITY;
    }
    Ok(())
}

fn update_contacts(
    collisions: Collisions,
    player_query: Query<Entity, With<Player>>,
    mut input_state: ResMut<MovementState>,
) -> Result<(), BevyError> {
    let player_entity = player_query.single()?;
    input_state.has_contact = false;
    for contact in collisions.iter() {
        let Some(entity_1) = contact.body1 else {
            continue;
        };
        let Some(entity_2) = contact.body2 else {
            continue;
        };
        if entity_1 == player_entity || entity_2 == player_entity {
            input_state.has_contact = true;
        }
    }
    Ok(())
}
