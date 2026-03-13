mod animation;
mod camera;
mod controller;
mod input;
mod inspector;
mod lighting;
mod loading;
mod markers;
mod mood;
mod scene;
mod splash;
mod state;

use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_skein::SkeinPlugin;

mod app_draw_layer {
    pub const MAIN: usize = 0;
    pub const HUD: usize = 1;
}

/// Sets running in PreUpdate related to inputs (polling them per frame,
/// updating things in the game according to inputus, and the effects to
/// apply after things have been updated according to new inputs).
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
enum InputSystems {
    PollInputs,
    ModifyStates,
    AfterStateUpdates,
}

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::srgb(0.2, 0.2, 0.2)))
        .add_plugins((
            DefaultPlugins,
            inspector::InspectorPlugin,
            PhysicsPlugins::default(),
            SkeinPlugin::default(),
            state::StatePlugin,
            animation::AnimationPlugin,
            camera::GameCameraPlugin,
            scene::ScenePlugin,
            input::InputPlugin,
            controller::CharacterControllerPlugin,
            lighting::LightingPlugin,
            loading::LoadingPlugin,
            markers::MarkerPlugin,
            mood::MoodPlugin,
            splash::SplashPlugin,
        ))
        .configure_sets(
            PreUpdate,
            (
                InputSystems::PollInputs,
                InputSystems::ModifyStates,
                InputSystems::AfterStateUpdates,
            )
                .chain(),
        )
        .run();
}
