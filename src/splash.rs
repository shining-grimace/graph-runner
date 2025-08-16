use crate::{
    app_draw_layer,
    loading::GameAssets,
    markers::{UiCamera, UiRoot},
    state::AppState,
};
use bevy::{prelude::*, render::view::RenderLayers};
use bevy_inspector_egui::bevy_egui::PrimaryEguiContext;

pub struct SplashPlugin;

impl Plugin for SplashPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Splash), spawn_splash_ui)
            .add_systems(OnExit(AppState::Splash), remove_splash_ui);
    }
}

fn spawn_splash_ui(mut commands: Commands, game_assets: Res<GameAssets>) {
    commands.spawn((
        UiCamera,
        Camera2d::default(),
        Camera {
            clear_color: ClearColorConfig::None,
            order: app_draw_layer::HUD as isize,
            ..default()
        },
        RenderLayers::layer(app_draw_layer::HUD),
        PrimaryEguiContext,
    ));
    commands
        .spawn((
            UiRoot,
            RenderLayers::layer(app_draw_layer::HUD),
            Node {
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                width: Val::Percent(50.0),
                height: Val::Percent(100.0),
                padding: UiRect {
                    left: Val::Percent(50.0),
                    ..default()
                },
                ..default()
            },
        ))
        .with_children(|parent| {
            parent.spawn((
                Node {
                    width: Val::Px(64.0),
                    height: Val::Px(64.0),
                    ..default()
                },
                ImageNode::new(game_assets.dev_logo.clone()),
            ));
            parent.spawn((
                Node {
                    padding: UiRect::all(Val::Px(16.0)),
                    ..default()
                },
                Text("MIDI Graph Demo".to_owned()),
                TextFont {
                    font_size: 48.0,
                    ..default()
                },
                TextColor(Color::srgb(0.8, 0.8, 0.1).into()),
                TextLayout {
                    justify: JustifyText::Center,

                    ..default()
                },
            ));
        });
}

fn remove_splash_ui(
    mut commands: Commands,
    ui_query: Query<Entity, With<UiRoot>>,
    camera_query: Query<Entity, With<UiCamera>>,
) -> Result<(), BevyError> {
    let ui = ui_query.single()?;
    commands.entity(ui).despawn();

    let camera = camera_query.single()?;
    commands.entity(camera).despawn();

    Ok(())
}
