use crate::{app_draw_layer, markers::Player, state::AppState};
use avian3d::schedule::PhysicsSet;
use bevy::{prelude::*, render::view::RenderLayers, transform::TransformSystem};
use bevy_inspector_egui::bevy_egui::PrimaryEguiContext;

const CAMERA_DISTANCE: f32 = 20.0;

pub struct GameCameraPlugin;

impl Plugin for GameCameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Game), spawn_camera)
            .add_systems(
                PostUpdate,
                update_camera
                    .run_if(in_state(AppState::Game))
                    .after(PhysicsSet::Sync)
                    .before(TransformSystem::TransformPropagate),
            );
    }
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Camera {
            order: app_draw_layer::MAIN as isize,
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, CAMERA_DISTANCE).looking_at(Vec3::ZERO, Vec3::Y),
        RenderLayers::layer(app_draw_layer::MAIN),
    ));
    commands.spawn((
        Camera2d::default(),
        Camera {
            order: app_draw_layer::HUD as isize,
            ..default()
        },
        RenderLayers::layer(app_draw_layer::HUD),
        PrimaryEguiContext,
    ));
}

fn update_camera(
    mut camera_query: Query<&mut Transform, (With<Camera3d>, Without<Player>)>,
    player_query: Query<&Transform, (With<Player>, Without<Camera>)>,
) -> Result<(), BevyError> {
    let mut camera_transform = camera_query.single_mut()?;
    let player_transform = player_query.single()?;
    camera_transform.translation.x = player_transform.translation.x;
    camera_transform.translation.y = player_transform.translation.y;
    Ok(())
}
