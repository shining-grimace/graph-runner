use crate::state::AppState;
use bevy::prelude::*;

pub struct LightingPlugin;

impl Plugin for LightingPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 1000.0,
            ..default()
        })
        .add_systems(OnEnter(AppState::Game), spawn_lights);
    }
}

fn spawn_lights(mut commands: Commands) {
    commands.spawn((PointLight::default(), Transform::from_xyz(0.0, 20.0, 0.0)));
}
