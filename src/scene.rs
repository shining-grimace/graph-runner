use crate::{loading::GameAssets, state::AppState};
use bevy::prelude::*;

pub struct ScenePlugin;

impl Plugin for ScenePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Game), initialise_game);
    }
}

fn initialise_game(mut commands: Commands, game_assets: Res<GameAssets>) -> Result<(), BevyError> {
    commands.spawn(SceneRoot(game_assets.models.clone()));
    Ok(())
}
