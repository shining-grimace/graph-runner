use bevy::{input::common_conditions::input_toggle_active, prelude::*};
use bevy_inspector_egui::{
    bevy_egui::{EguiGlobalSettings, EguiPlugin},
    quick::WorldInspectorPlugin,
};

/// Initially the inspector is hidden; press left-control to toggle visibility
pub struct InspectorPlugin;

impl Plugin for InspectorPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            EguiPlugin::default(),
            WorldInspectorPlugin::new().run_if(input_toggle_active(false, KeyCode::ControlLeft)),
        ))
        .insert_resource(EguiGlobalSettings {
            auto_create_primary_context: false,
            ..default()
        });
    }
}
