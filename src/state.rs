
use bevy::prelude::*;
use std::time::Duration;

const SPLASH_DURATION_SECS: u64 = 3;

#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
pub enum AppState {
    #[default]
    Loading,
    Splash,
    Game
}

#[derive(Resource, Default)]
pub struct AppStateStartTime(Duration);

pub struct StatePlugin;

impl Plugin for StatePlugin {
    fn build(&self, app: &mut App) {
        app
            .init_state::<AppState>()
            .init_resource::<AppStateStartTime>()
            .add_systems(OnEnter(AppState::Loading), on_enter_state)
            .add_systems(OnEnter(AppState::Splash), on_enter_state)
            .add_systems(OnEnter(AppState::Game), on_enter_state)
            .add_systems(Update, leave_splash_after_delay
                .run_if(in_state(AppState::Splash)));
    }
}

fn on_enter_state(
    mut app_state_start_time: ResMut<AppStateStartTime>,
    time: Res<Time>
) {
    app_state_start_time.0 = time.elapsed();
}

fn leave_splash_after_delay(
    mut next_app_state: ResMut<NextState<AppState>>,
    app_state_start_time: Res<AppStateStartTime>,
    time: Res<Time>
) {
    let state_duration = time.elapsed() - app_state_start_time.0;
    if state_duration > Duration::from_secs(SPLASH_DURATION_SECS) {
        next_app_state.set(AppState::Game);
    }
}

