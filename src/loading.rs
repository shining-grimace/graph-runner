use crate::{markers::UiRoot, state::AppState};
use bevy::{asset::LoadState, prelude::*, render::view::RenderLayers};

pub const MESH_NAME_PLAYER: &str = "Player";
pub const MESH_NAME_MAP: &str = "Map";

#[derive(Resource, Default)]
pub struct GameAssets {
    pub dev_logo: Handle<Image>,
    pub models: Handle<Gltf>,
}

impl GameAssets {
    fn get_all_file_assets_untyped(&self) -> [UntypedHandle; 2] {
        [
            self.dev_logo.clone_weak().untyped(),
            self.models.clone_weak().untyped(),
        ]
    }
}

pub struct LoadingPlugin;

impl Plugin for LoadingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GameAssets>()
            .add_systems(
                OnEnter(AppState::Loading),
                (spawn_loading_ui, init_game_assets),
            )
            .add_systems(OnExit(AppState::Loading), remove_loading_ui)
            .add_systems(
                Update,
                check_game_assets_ready.run_if(in_state(AppState::Loading)),
            );
    }
}

fn spawn_loading_ui(mut commands: Commands) {
    commands.spawn((
        Camera2d::default(),
        Camera {
            clear_color: ClearColorConfig::None,
            order: 1,
            ..default()
        },
        RenderLayers::layer(1),
    ));

    commands
        .spawn((
            UiRoot,
            Node {
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::End,
                align_items: AlignItems::End,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..Node::DEFAULT
            },
        ))
        .with_children(|parent| {
            parent.spawn((
                Node {
                    padding: UiRect::all(Val::Px(16.0)),
                    ..default()
                },
                Text("Loading...".to_owned()),
                TextFont {
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::WHITE.into()),
                TextLayout {
                    justify: JustifyText::Center,
                    ..default()
                },
            ));
        });
}

fn remove_loading_ui(
    mut commands: Commands,
    ui_query: Query<Entity, With<UiRoot>>,
    camera_query: Query<Entity, With<Camera>>,
) -> Result<(), BevyError> {
    let ui = ui_query.single()?;
    commands.entity(ui).despawn();

    let camera = camera_query.single()?;
    commands.entity(camera).despawn();

    Ok(())
}

fn init_game_assets(ass: Res<AssetServer>, mut game_assets: ResMut<GameAssets>) {
    game_assets.dev_logo = ass.load("images/shining-grimace-logo.png");
    game_assets.models = ass.load("models/models.glb");
}

fn check_game_assets_ready(
    ass: Res<AssetServer>,
    game_assets: Res<GameAssets>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    let handles = game_assets.get_all_file_assets_untyped();
    for (index, handle) in handles.iter().enumerate() {
        if !is_ready(&ass, &handle, index) {
            return;
        }
    }
    next_state.set(AppState::Splash);
}

fn is_ready(ass: &Res<AssetServer>, handle: &UntypedHandle, index: usize) -> bool {
    match ass.load_state(handle.id()) {
        LoadState::Failed(error) => panic!("Asset load failed at index {}: {:?}", index, error),
        LoadState::NotLoaded => panic!("Asset not loading at index {}", index),
        LoadState::Loaded | LoadState::Loading => ass.is_loaded_with_dependencies(handle.id()),
    }
}
