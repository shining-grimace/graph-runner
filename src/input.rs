use crate::state::AppState;
use bevy::prelude::*;

#[derive(Default, Resource)]
pub struct MovementState {
    pub input_direction_x: f32,
    pub pressing_jump: bool,
    pub just_pressed_jump: bool,
}

impl MovementState {
    pub fn reset(&mut self) {
        self.input_direction_x = 0.0;
        self.pressing_jump = false;
        self.just_pressed_jump = false;
    }
}

pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(MovementState::default())
            .add_systems(PreUpdate, poll_inputs.run_if(in_state(AppState::Game)));
    }
}

fn poll_inputs(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut input_state: ResMut<MovementState>,
    mut exit_signal: MessageWriter<AppExit>,
) {
    if keyboard_input.just_pressed(KeyCode::Escape) {
        exit_signal.write(AppExit::Success);
        return;
    }

    let was_previously_pressing_jump = input_state.pressing_jump;
    input_state.reset();

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

    let now_pressing_jump =
        keyboard_input.pressed(KeyCode::Space) || keyboard_input.pressed(KeyCode::KeyL);
    input_state.pressing_jump = now_pressing_jump;
    input_state.just_pressed_jump = now_pressing_jump && !was_previously_pressing_jump;
}
