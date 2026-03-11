use avian3d::{
    math::{Scalar, Vector},
    prelude::*,
};
use bevy::{
    ecs::{lifecycle::HookContext, world::DeferredWorld},
    prelude::*,
};

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct UiRoot;

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct UiCamera;

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct Player;

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct SpawnPoint;

#[derive(Component, Reflect)]
#[reflect(Component)]
#[component(storage = "SparseSet")]
pub struct Grounded {
    pub normal: Vector,
    pub distance: Scalar,
}

#[derive(Component, Reflect, Debug)]
#[reflect(Component)]
#[component(on_add = on_water_volume_added)]
pub struct WaterVolume;

#[derive(Component, Reflect, Debug)]
#[reflect(Component)]
pub struct WaterVolumeExtents {
    pub half_extent_x: f32,
    pub half_extent_y: f32,
    pub half_extent_z: f32,
}

#[derive(Component, Reflect)]
#[reflect(Component)]
#[component(on_add = on_trimesh_added)]
pub struct Trimesh;

pub struct MarkerPlugin;

impl Plugin for MarkerPlugin {
    fn build(&self, _app: &mut App) {}
}

fn on_trimesh_added(mut world: DeferredWorld, context: HookContext) {
    let collider = {
        let mesh_3d = world.entity(context.entity).get::<Mesh3d>();
        match mesh_3d {
            Some(mesh_3d) => {
                let meshes = world.get_resource::<Assets<Mesh>>().unwrap();
                let mesh = meshes.get(&mesh_3d.0).unwrap();
                Some(Collider::trimesh_from_mesh(mesh).unwrap())
            }
            None => {
                eprintln!(
                    "Trimesh entity {} must have a Mesh3d component",
                    context.entity
                );
                None
            }
        }
    };
    if let Some(collider) = collider {
        world.commands().entity(context.entity).insert(collider);
    };
}

fn on_water_volume_added(mut world: DeferredWorld, context: HookContext) {
    let mesh_3d = world.entity(context.entity).get::<Mesh3d>();
    let Some(mesh_3d) = mesh_3d else {
        eprintln!(
            "WaterVolume entity {} must have a Mesh3d component",
            context.entity
        );
        return;
    };
    let meshes = world.get_resource::<Assets<Mesh>>().unwrap();
    let mesh = meshes.get(&mesh_3d.0).unwrap();
    let positions = mesh
        .attribute(Mesh::ATTRIBUTE_POSITION)
        .expect("A water mesh doesn't have a position attribute")
        .as_float3()
        .expect("A water mesh's position attribute isn't stored as float3");

    let mut min_x: f32 = 0.0;
    let mut max_x: f32 = 0.0;
    let mut min_y: f32 = 0.0;
    let mut max_y: f32 = 0.0;
    let mut min_z: f32 = 0.0;
    let mut max_z: f32 = 0.0;
    for vertex in positions.iter() {
        min_x = min_x.min(vertex[0]);
        max_x = max_x.max(vertex[0]);
        min_y = min_y.min(vertex[1]);
        max_y = max_y.max(vertex[1]);
        min_z = min_z.min(vertex[2]);
        max_z = max_z.max(vertex[2]);
    }
    if max_x == 0.0 || max_y == 0.0 || max_z == 0.0 {
        panic!("Water volume extents are unbalanced around the origin!");
    }
    if ((min_x / max_x) + 1.0).abs() > 0.01 {
        panic!("Water volume extents are unbalanced around the origin!");
    }
    if ((min_y / max_y) + 1.0).abs() > 0.01 {
        panic!("Water volume extents are unbalanced around the origin!");
    }
    if ((min_z / max_z) + 1.0).abs() > 0.01 {
        panic!("Water volume extents are unbalanced around the origin!");
    }
    println!(
        "Processed WaterVolume with extents ({}, {}), ({}, {}), ({}, {})",
        min_x, max_x, min_y, max_y, min_z, max_z
    );
    world
        .commands()
        .entity(context.entity)
        .insert(WaterVolumeExtents {
            half_extent_x: max_x,
            half_extent_y: max_y,
            half_extent_z: max_z,
        });
}
