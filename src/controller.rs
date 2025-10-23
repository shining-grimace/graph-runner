use crate::{
    game::PLAYER_HEIGHT,
    input::MovementState,
    markers::{Grounded, Player},
    state::AppState,
};
use avian3d::{
    math::{Quaternion, Scalar, Vector},
    prelude::*,
};
use bevy::prelude::*;

#[derive(Component, Reflect, Default)]
#[reflect(Component, Default)]
#[require(
    Player = Player,
    Collider = PlayerController::collider(0.0),
    RigidBody::Static, // Includes LinearVelocity
    LockedAxes::ROTATION_LOCKED
)]
pub struct PlayerController {
    velocity: Vector,
}

impl PlayerController {
    /// Build a collider, reducing by a skin thickness
    pub fn collider(skin_thickness: f32) -> Collider {
        let player_radius = 0.4;
        Collider::cylinder(
            player_radius - skin_thickness,
            PLAYER_HEIGHT - 2.0 * skin_thickness,
        )
    }
}

#[derive(Resource)]
pub struct CharacterControllerParams {
    pub movement_acceleration: Scalar,
    pub movement_damping_factor: Scalar,
    pub jump_impulse: Scalar,
    pub gravity: Vector,
    pub collider_skin_thickness: Scalar,
    pub max_collision_bounces: usize,
    pub max_slope_angle: Scalar,
    pub move_max_speed: Scalar,
}

impl Default for CharacterControllerParams {
    fn default() -> Self {
        Self {
            movement_acceleration: 30.0,
            movement_damping_factor: 0.9,
            jump_impulse: 12.0,
            gravity: Vector::NEG_Y * 9.81 * 2.0,
            collider_skin_thickness: 0.01,
            max_collision_bounces: 3,
            max_slope_angle: std::f32::consts::PI * 0.45,
            move_max_speed: 8.0,
        }
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
        app.register_type::<PlayerController>()
            .insert_resource(CharacterControllerParams::default())
            .add_systems(
                Update,
                (apply_gravity, apply_inputs, apply_movement_damping)
                    .chain()
                    .run_if(in_state(AppState::Game)),
            )
            .add_systems(
                // The Avian schedule runs in the fixed timestep schedule.
                // Fixed timestep runs zero or more times before Update.
                // Also note Position should be managed in these systems rather than Transform
                // because they're required to run in the SubstepSchedule (to run spatial queries).
                PhysicsSchedule,
                (move_player, update_grounded)
                    .chain()
                    .in_set(PhysicsStepSet::Last)
                    .run_if(in_state(AppState::Game)),
            );
    }
}

fn apply_gravity(
    mut query: Query<&mut PlayerController>,
    params: Res<CharacterControllerParams>,
    time: Res<Time>,
) -> Result<(), BevyError> {
    let delta_time = time.delta_secs();
    let Ok(mut controller) = query.single_mut() else {
        println!("Not running apply_gravity this timestep");
        return Ok(());
    };
    controller.velocity += params.gravity * delta_time;
    Ok(())
}

fn apply_inputs(
    mut query: Query<(&mut PlayerController, Has<Grounded>)>,
    params: Res<CharacterControllerParams>,
    inputs: Res<MovementState>,
    time: Res<Time>,
) -> Result<(), BevyError> {
    let delta_time = time.delta_secs();
    let Ok((mut controller, is_grounded)) = query.single_mut() else {
        println!("Not running apply_inputs this timestep");
        return Ok(());
    };
    controller.velocity.x = (controller.velocity.x
        + inputs.input_direction_x * params.movement_acceleration * delta_time)
        .min(params.move_max_speed)
        .max(-params.move_max_speed);
    controller.velocity.z = 0.0;
    if inputs.just_pressed_jump && is_grounded {
        controller.velocity.y += params.jump_impulse;
    }
    Ok(())
}

/// Slows down movement in the X direction
fn apply_movement_damping(
    mut query: Query<&mut PlayerController>,
    params: Res<CharacterControllerParams>,
) -> Result<(), BevyError> {
    let Ok(mut controller) = query.single_mut() else {
        println!("Not running apply_movement_damping this timestep");
        return Ok(());
    };
    controller.velocity.x *= params.movement_damping_factor;
    Ok(())
}

/// Kinematic bodies don't get pushed by collisions by default, allowing a custom resolver to work.
fn move_player(
    mut controllers_query: Query<(Entity, &mut Transform, &mut PlayerController)>,
    spatial_queries: Res<SpatialQueryPipeline>,
    params: Res<CharacterControllerParams>,
    time: Res<Time>,
) -> Result<(), BevyError> {
    let Ok((entity, mut transform, mut controller)) = controllers_query.single_mut() else {
        println!("Not running move_player this timestep");
        return Ok(());
    };
    let shape = PlayerController::collider(params.collider_skin_thickness);
    let attempted_displacement = time.delta_secs() * controller.velocity;

    let new_position = move_and_collide_and_slide(
        spatial_queries,
        params,
        &shape,
        entity,
        transform.translation,
        attempted_displacement,
        0,
    );
    let travel = new_position - transform.translation;
    transform.translation = new_position;
    controller.velocity = travel / time.delta_secs();
    Ok(())
}

/// Update the [`Grounded`] status for the [`PlayerController`].
/// The player is grounded if the shape cast has a hit with a normal that isn't too steep.
fn update_grounded(
    mut commands: Commands,
    mut query: Query<(Entity, &Transform, &Rotation), With<PlayerController>>,
    spatial_queries: Res<SpatialQueryPipeline>,
    params: Res<CharacterControllerParams>,
) -> Result<(), BevyError> {
    let Ok((entity, transform, rotation)) = query.single_mut() else {
        println!("Not running update_grounded this timestep");
        return Ok(());
    };

    let shape = PlayerController::collider(0.0);
    let origin = transform.translation;
    let shape_rotation = Quaternion::default();
    let direction = Dir3::NEG_Y;
    let config = ShapeCastConfig {
        max_distance: 0.4,
        ..default()
    };
    let filter = SpatialQueryFilter::default().with_excluded_entities([entity]);
    let closest_hit =
        spatial_queries.cast_shape(&shape, origin, shape_rotation, direction, &config, &filter);

    let grounding = match closest_hit {
        Some(hit) => {
            let world_normal = rotation * -hit.normal2;
            if world_normal.angle_between(Vector::Y).abs() <= params.max_slope_angle {
                Some(Grounded {
                    normal: world_normal,
                    distance: hit.distance,
                })
            } else {
                None
            }
        }
        None => None,
    };

    if let Some(grounding) = grounding {
        commands.entity(entity).insert(grounding);
    } else {
        commands.entity(entity).remove::<Grounded>();
    }

    Ok(())
}

fn move_and_collide_and_slide(
    spatial_queries: Res<SpatialQueryPipeline>,
    params: Res<CharacterControllerParams>,
    shape: &Collider,
    entity: Entity,
    position: Vec3,
    attempted_displacement: Vec3,
    bounce_no: usize,
) -> Vec3 {
    let shape_rotation = Quaternion::default();
    let Ok(direction) = Dir3::new(attempted_displacement) else {
        return position + attempted_displacement;
    };
    let config = ShapeCastConfig {
        max_distance: attempted_displacement.length() + params.collider_skin_thickness,
        ..default()
    };
    let filter = SpatialQueryFilter::default().with_excluded_entities([entity]);
    let closest_hit = spatial_queries.cast_shape(
        &shape,
        position,
        shape_rotation,
        direction,
        &config,
        &filter,
    );
    match closest_hit {
        None => position + attempted_displacement,
        Some(hit) => {
            if hit.distance == 0.0 {
                println!("WARNING: Looks like the player is intersecting a static body!")
            }
            let travel = (hit.distance - params.collider_skin_thickness) * direction;
            let remaining_displacement = attempted_displacement - travel;
            let remaining_along_normal = remaining_displacement.project_onto(hit.normal2);
            let slide_displacement = remaining_displacement - remaining_along_normal;
            let collision_position = position + travel;
            if bounce_no + 1 < params.max_collision_bounces {
                let next_position = move_and_collide_and_slide(
                    spatial_queries,
                    params,
                    shape,
                    entity,
                    collision_position,
                    slide_displacement,
                    bounce_no + 1,
                );
                next_position
            } else {
                collision_position
            }
        }
    }
}
