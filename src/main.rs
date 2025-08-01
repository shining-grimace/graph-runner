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
use bevy::{input::common_conditions::input_toggle_active, prelude::*};
use bevy_inspector_egui::{bevy_egui::EguiPlugin, quick::WorldInspectorPlugin};
use bevy_skein::SkeinPlugin;

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::srgb(0.2, 0.2, 0.2)))
        .add_plugins((
            DefaultPlugins,
            EguiPlugin {
                enable_multipass_for_primary_context: true,
            },
            WorldInspectorPlugin::new().run_if(input_toggle_active(true, KeyCode::ControlLeft)),
            PhysicsPlugins::default(),
            SkeinPlugin::default(),
            state::StatePlugin,
            camera::GameCameraPlugin,
            game::GamePlugin,
            input::InputPlugin,
            controller::CharacterControllerPlugin,
            lighting::LightingPlugin,
            loading::LoadingPlugin,
            markers::MarkerPlugin,
            mood::MoodPlugin,
            splash::SplashPlugin,
        ))
        .run();
}
