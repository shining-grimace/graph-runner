use crate::{InputSystems, state::AppState};
use bevy::prelude::*;

#[derive(Default, Resource)]
pub struct MovementState {
    // Directional movement
    pub input_direction_x: f32,
    pub input_direction_y: f32,

    // Jump input
    pub pressing_jump: bool,
    pub just_pressed_jump: bool,

    // Secondary input
    pub pressing_secondary: bool,
    pub just_pressed_secondary: bool,
}

#[cfg(debug_assertions)]
#[derive(Event)]
pub struct DebugPressed;

pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(MovementState::default()).add_systems(
            PreUpdate,
            poll_inputs
                .in_set(InputSystems::PollInputs)
                .run_if(in_state(AppState::Game)),
        );
    }
}

fn poll_inputs(
    #[cfg(debug_assertions)] mut commands: Commands,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut input_state: ResMut<MovementState>,
    mut exit_signal: MessageWriter<AppExit>,
) {
    if keyboard_input.just_pressed(KeyCode::Escape) {
        exit_signal.write(AppExit::Success);
        return;
    }

    let was_previously_pressing_jump = input_state.pressing_jump;
    let was_previously_pressing_secondary = input_state.pressing_secondary;
    *input_state = MovementState::default();

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

    let pressing_up =
        keyboard_input.pressed(KeyCode::ArrowUp) || keyboard_input.pressed(KeyCode::KeyW);
    let pressing_down =
        keyboard_input.pressed(KeyCode::ArrowDown) || keyboard_input.pressed(KeyCode::KeyS);
    input_state.input_direction_y = if pressing_up && pressing_down {
        0.0
    } else if pressing_up {
        1.0
    } else if pressing_down {
        -1.0
    } else {
        0.0
    };

    let now_pressing_jump =
        keyboard_input.pressed(KeyCode::Space) || keyboard_input.pressed(KeyCode::KeyL);
    input_state.pressing_jump = now_pressing_jump;
    input_state.just_pressed_jump = now_pressing_jump && !was_previously_pressing_jump;

    let now_pressing_secondary =
        keyboard_input.pressed(KeyCode::ShiftLeft) || keyboard_input.pressed(KeyCode::KeyK);
    input_state.pressing_secondary = now_pressing_secondary;
    input_state.just_pressed_secondary =
        now_pressing_secondary && !was_previously_pressing_secondary;

    #[cfg(debug_assertions)]
    if keyboard_input.just_pressed(KeyCode::KeyB) {
        commands.trigger(DebugPressed);
    }
}
