mod functions;
mod math;
mod params;
mod systems;

use crate::markers::Player;
use avian3d::{
    math::{Scalar, Vector},
    prelude::*,
};
use bevy::{
    ecs::{lifecycle::HookContext, world::DeferredWorld},
    prelude::*,
};

pub const PLAYER_HEIGHT: f32 = 1.50;
pub const PLAYER_RADIUS: f32 = 0.5;

const GROUNDING_PROXIMITY: Scalar = 0.4;
const WALL_RETENTION_PROXIMITY: Scalar = 0.1;

struct MovementResult {
    new_position: Vec3,
    new_attachment: Option<(Attachment, Option<SpecialMove>)>,
    new_velocity: Option<Vector>,
}

#[derive(Component, Reflect, Debug)]
#[reflect(Component)]
#[component(
    storage = "SparseSet",
    on_insert = Self::on_insert,
    on_remove = Self::on_remove
)]
pub enum Attachment {
    Grounded { normal: Vector },
    LedgeGrabbed { normal: Vector },
    Walled { normal: Vector, progress: f32 },
    Submerged { water_volume_entity: Entity },
    Floating { water_volume_entity: Entity },
}

impl Attachment {
    fn on_insert(world: DeferredWorld, context: HookContext) {
        let value = world.get::<Self>(context.entity).unwrap();
        let progress = match value {
            Attachment::Grounded { .. } => 0.0,
            Attachment::LedgeGrabbed { .. } => 0.0,
            Attachment::Walled { progress, .. } => *progress,
            Attachment::Submerged { .. } => 0.0,
            Attachment::Floating { .. } => 0.0,
        };
        if progress != 0.0 {
            return;
        }
        println!("{:?} attachment inserted on player controller", value);
    }

    fn on_remove(world: DeferredWorld, context: HookContext) {
        let value = world.get::<Self>(context.entity).unwrap();
        println!("{:?} attachment removed from player controller", value);
    }
}

#[derive(Component, Reflect, Debug)]
#[reflect(Component)]
#[component(
    storage = "SparseSet",
    on_insert = Self::on_insert,
    on_remove = Self::on_remove
)]
pub enum SpecialMove {
    Landing { progress: f32 },
    Rolling { progress: f32 },
    Running,
    Halting { progress: f32 },
    Sliding,
    Jumping,
    Diving,
    Climbing { progress: f32, normal: Vec3 },
}

impl SpecialMove {
    fn on_insert(world: DeferredWorld, context: HookContext) {
        let value = world.get::<Self>(context.entity).unwrap();
        let progress = match value {
            SpecialMove::Landing { progress } => *progress,
            SpecialMove::Rolling { progress } => *progress,
            SpecialMove::Running => 0.0,
            SpecialMove::Halting { progress } => *progress,
            SpecialMove::Sliding => 0.0,
            SpecialMove::Jumping => 0.0,
            SpecialMove::Diving => 0.0,
            SpecialMove::Climbing { progress, .. } => *progress,
        };
        if progress != 0.0 {
            return;
        }
        println!("{:?} special move inserted on player controller", value);
    }

    fn on_remove(world: DeferredWorld, context: HookContext) {
        let value = world.get::<Self>(context.entity).unwrap();
        println!("{:?} special move removed from player controller", value);
    }
}

#[derive(Reflect)]
pub struct Manoeuvrability {
    /// Horizontal acceleration caused by input
    pub input_factor: Scalar,

    /// Horizontal deceleration caused by input
    pub reverse_input_factor: Scalar,

    /// Vertical impulse of jumping
    pub jump_factor: Scalar,

    /// Maximum horizontal speed
    pub max_speed_factor: Scalar,

    /// The time it takes to stop horizontally when inputs are released
    pub stop_factor: Scalar,
}

#[derive(Reflect)]
pub struct HitProperties {
    pub normal: Vector,
    pub distance: f32,
    pub normal_angle: f32,
    pub entity: Entity,
}

impl HitProperties {
    pub fn from_avian_hit(
        hit: &ShapeHitData,
        rotation: &Rotation,
        internal_cast_distance: f32,
    ) -> Self {
        let normal = rotation * -hit.normal2;
        Self {
            distance: hit.distance - internal_cast_distance,
            normal_angle: normal.angle_between(Vector::Y),
            normal,
            entity: hit.entity,
        }
    }
}

#[derive(Component, Default, Reflect)]
#[reflect(Component, Default)]
pub struct PlayerHits {
    ground: Option<HitProperties>,
    water_volume: Option<Entity>,
    water_surface: Option<HitProperties>,
    ledge: Option<HitProperties>,
    wall: Option<HitProperties>,
}

#[derive(Component)]
pub struct Facing {
    angle: f32,
}

impl Default for Facing {
    fn default() -> Self {
        Self {
            angle: std::f32::consts::FRAC_PI_2,
        }
    }
}

#[derive(Resource)]
pub struct PlayerAndWaterEntities {
    pub entities: Vec<Entity>,
}

impl PlayerAndWaterEntities {
    fn from_entities(entities: &[Entity]) -> Self {
        Self {
            entities: entities.iter().copied().collect(),
        }
    }

    fn excluded_entities(&self) -> impl core::iter::IntoIterator<Item = Entity> {
        self.entities.iter().copied()
    }
}

#[derive(Component, Reflect, Default)]
#[reflect(Component, Default)]
#[require(
    Player = Player,
    Collider = PlayerController::collider(0.0),
    Facing,
    PlayerHits,
    RigidBody::Static, // Includes LinearVelocity
    LockedAxes::ROTATION_LOCKED,
    NoTranslationEasing
)]
pub struct PlayerController {
    velocity: Vector,
}

impl PlayerController {
    /// Build a collider, reducing by a skin thickness
    pub fn collider(skin_thickness: f32) -> Collider {
        Collider::cylinder(
            PLAYER_RADIUS - skin_thickness,
            PLAYER_HEIGHT - 2.0 * skin_thickness,
        )
    }
}

/// Character controller initially based on the fairly simple kinematic character controller
/// example provided by Avian, but adapted according to a nice video on the Collide and Slide
/// algorithm.
///
/// See:
/// https://github.com/Jondolf/avian/blob/main/crates/avian3d/examples/kinematic_character_3d/plugin.rs
/// https://www.youtube.com/watch?v=YR6Q7dUz2uk
///
///
/// Notes on Avian:
/// The LinearVelocity component is manipulated a fair bit by Avian, so this controller uses a
/// separate component for velocity that's retained across frames.
/// Furthermore, a non-static RigidBody will have its position updated by Avian in the [SolverSet]
/// (by calculating [AccumulatedTranslation] as LinearVelocity over time delta, and adding that to the
/// Position which is later written back to the Translation. All this is avoided here by using a static
/// RigidBody. See [SolverPlugin] and [IntegratorPlugin].
///
/// Notes on using Skein:
/// Be careful which entities have components; a Blender model will have an entity for the object, and
/// a child entity for the mesh, so the player should have components inserted on the object while a
/// trimesh should have the Trimesh marker inserted on the mesh.
pub struct CharacterControllerPlugin;

impl Plugin for CharacterControllerPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(PlayerAndWaterEntities::from_entities(&[]))
            .insert_resource(params::CharacterControllerParams::default());
        systems::schedule_systems(app);
    }
}
