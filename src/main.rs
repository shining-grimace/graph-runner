mod camera;
mod controller;
mod game;
mod input;
mod lighting;
mod loading;
mod markers;
mod mood;
mod splash;
mod state;

use avian3d::prelude::*;
use bevy::prelude::*;

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::srgb(0.2, 0.2, 0.2)))
        .add_plugins((
            DefaultPlugins,
            PhysicsPlugins::default(),
            state::StatePlugin,
            camera::GameCameraPlugin,
            game::GamePlugin,
            input::InputPlugin,
            controller::CharacterControllerPlugin,
            lighting::LightingPlugin,
            loading::LoadingPlugin,
            mood::MoodPlugin,
            splash::SplashPlugin,
        ))
        .run();
}
