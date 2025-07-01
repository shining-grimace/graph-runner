use crate::{
    loading::{GameAssets, MESH_NAME_MAP, MESH_NAME_PLAYER},
    markers::Player,
    state::AppState,
};
use avian3d::prelude::*;
use bevy::{gltf::GltfMesh, prelude::*};

const PLAYER_HEIGHT: f32 = 1.74;

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Game), initialise_game);
    }
}

fn initialise_game(
    mut commands: Commands,
    game_assets: Res<GameAssets>,
    mut material_assets: ResMut<Assets<StandardMaterial>>,
    gltf_assets: Res<Assets<Gltf>>,
    gltf_mesh_assets: Res<Assets<GltfMesh>>,
    mesh_assets: Res<Assets<Mesh>>,
) -> Result<(), BevyError> {
    let gltf_data = gltf_assets
        .get(&game_assets.models)
        .ok_or_else(|| "glTF asset not loaded")?;
    let player_mesh_handle = gltf_data
        .named_meshes
        .get(MESH_NAME_PLAYER)
        .ok_or_else(|| "Player glTF mesh handle not found")?;
    let player_gltf_mesh = gltf_mesh_assets
        .get(player_mesh_handle)
        .ok_or_else(|| "Player glTF mesh not loaded")?;
    let map_mesh_handle = gltf_data
        .named_meshes
        .get(MESH_NAME_MAP)
        .ok_or_else(|| "Map glTF mesh handle not found")?;
    let map_gltf_mesh = gltf_mesh_assets
        .get(map_mesh_handle)
        .ok_or_else(|| "Map glTF mesh not loaded")?;
    let map_mesh = mesh_assets
        .get(&map_gltf_mesh.primitives[0].mesh)
        .ok_or_else(|| "Map mesh not loaded")?;
    commands.spawn((
        Player,
        RigidBody::Dynamic,
        Collider::cylinder(0.4, PLAYER_HEIGHT),
        LockedAxes::ROTATION_LOCKED,
        LinearVelocity::default(),
        Transform::from_xyz(0.0, 0.5 * PLAYER_HEIGHT + 0.05, 0.0),
        Mesh3d(player_gltf_mesh.primitives[0].mesh.clone()),
        MeshMaterial3d(material_assets.add(StandardMaterial {
            base_color: Color::srgb(1.0, 0.4, 0.4),
            ..default()
        })),
    ));
    commands.spawn((
        RigidBody::Static,
        Collider::trimesh_from_mesh(&map_mesh).unwrap(),
        Transform::default(),
        Mesh3d(map_gltf_mesh.primitives[0].mesh.clone()),
        MeshMaterial3d(material_assets.add(StandardMaterial {
            base_color: Color::srgb(0.4, 1.0, 0.4),
            ..default()
        })),
    ));
    Ok(())
}
