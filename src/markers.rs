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
#[component(storage = "SparseSet")]
pub struct Grounded {
    pub normal: Vector,
    pub distance: Scalar,
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
