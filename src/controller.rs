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

#[derive(Component, Reflect)]
#[reflect(Component)]
#[require(
    Player = Player,
    Collider = PlayerController::collider(),
    ShapeCaster = PlayerController::shape_caster(),
    RigidBody::Kinematic, // Includes LinearVelocity
    LockedAxes::ROTATION_LOCKED
)]
pub struct PlayerController;

impl PlayerController {
    pub fn collider() -> Collider {
        Collider::cylinder(0.4, PLAYER_HEIGHT)
    }

    pub fn shape_caster() -> ShapeCaster {
        ShapeCaster::new(
            Self::collider(),
            Vector::ZERO,
            Quaternion::default(),
            Dir3::NEG_Y,
        )
        .with_max_distance(0.4)
    }
}

#[derive(Resource)]
pub struct CharacterControllerParams {
    pub movement_acceleration: Scalar,
    pub movement_damping_factor: Scalar,
    pub jump_impulse: Scalar,
    pub gravity: Vector,
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
            max_slope_angle: std::f32::consts::PI * 0.45,
            move_max_speed: 8.0,
        }
    }
}

/// Character controller based on the fairly simple kinematic character controller
/// example provided by Avian.
///
/// See:
/// https://github.com/Jondolf/avian/blob/main/crates/avian3d/examples/kinematic_character_3d/plugin.rs
pub struct CharacterControllerPlugin;

impl Plugin for CharacterControllerPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<PlayerController>()
            .insert_resource(CharacterControllerParams::default())
            .add_systems(
                Update,
                (
                    update_grounded,
                    apply_gravity,
                    apply_inputs,
                    apply_movement_damping,
                )
                    .chain()
                    .run_if(in_state(AppState::Game)),
            )
            .add_systems(
                PhysicsSchedule,
                controller_collisions
                    .in_set(NarrowPhaseSet::Last)
                    .run_if(in_state(AppState::Game)),
            );
    }
}

/// Update the [`Grounded`] status for the [`PlayerController`].
/// The player is grounded if the shape caster has a hit with a normal that isn't too steep.
fn update_grounded(
    mut commands: Commands,
    mut query: Query<(Entity, &ShapeHits, &Rotation), With<PlayerController>>,
    params: Res<CharacterControllerParams>,
) {
    for (entity, hits, rotation) in &mut query {
        let is_grounded = hits.iter().any(|hit| {
            (rotation * -hit.normal2).angle_between(Vector::Y).abs() <= params.max_slope_angle
        });

        if is_grounded {
            commands.entity(entity).insert(Grounded);
        } else {
            commands.entity(entity).remove::<Grounded>();
        }
    }
}

fn apply_gravity(
    mut query: Query<&mut LinearVelocity, With<PlayerController>>,
    params: Res<CharacterControllerParams>,
    time: Res<Time>,
) {
    let delta_time = time.delta_secs();
    for mut velocity in &mut query {
        velocity.0 += params.gravity * delta_time;
    }
}

fn apply_inputs(
    mut query: Query<(&mut LinearVelocity, Has<Grounded>), With<PlayerController>>,
    params: Res<CharacterControllerParams>,
    inputs: Res<MovementState>,
    time: Res<Time>,
) {
    let delta_time = time.delta_secs();
    for (mut velocity, is_grounded) in &mut query {
        velocity.x = (velocity.x
            + inputs.input_direction_x * params.movement_acceleration * delta_time)
            .min(params.move_max_speed)
            .max(-params.move_max_speed);
        velocity.z = 0.0;
        if inputs.just_pressed_jump && is_grounded {
            velocity.y += params.jump_impulse;
        }
    }
}

/// Slows down movement in the X direction
fn apply_movement_damping(
    mut query: Query<&mut LinearVelocity, With<PlayerController>>,
    params: Res<CharacterControllerParams>,
) {
    for mut velocity in &mut query {
        velocity.x *= params.movement_damping_factor;
    }
}

/// Kinematic bodies don't get pushed by collisions by default, so it must be done manually.
///
/// This system handles collision response by pushing bodies along their contact normal by
/// their penetration depth, and correcting velocities in order to snap to slopes, slide along
/// walls, and predict collisions using speculative contacts.
fn controller_collisions(
    collisions: Collisions,
    bodies_query: Query<&RigidBody>,
    collider_rbs_query: Query<&ColliderOf, Without<Sensor>>,
    mut controllers_query: Query<(&mut Position, &mut LinearVelocity), With<PlayerController>>,
    params: Res<CharacterControllerParams>,
    time: Res<Time>,
) {
    // Iterate through collisions and resolve body penetration
    for contacts in collisions.iter() {
        // Get the rigid body entities of the colliders (colliders could be children)
        let Ok([&ColliderOf { body: rb1 }, &ColliderOf { body: rb2 }]) =
            collider_rbs_query.get_many([contacts.collider1, contacts.collider2])
        else {
            continue;
        };

        // Get the body of the player controller and whether it's the first or second entity
        // in the collision
        let is_first: bool;
        let is_other_dynamic: bool;
        let (mut position, mut velocity) = if let Ok(character) = controllers_query.get_mut(rb1) {
            is_first = true;
            is_other_dynamic = bodies_query.get(rb2).is_ok_and(|rb| rb.is_dynamic());
            character
        } else if let Ok(character) = controllers_query.get_mut(rb2) {
            is_first = false;
            is_other_dynamic = bodies_query.get(rb1).is_ok_and(|rb| rb.is_dynamic());
            character
        } else {
            continue;
        };

        // Iterate through contact manifolds and their contacts (each contact in a manifold
        // shares the same contact normal)
        for manifold in contacts.manifolds.iter() {
            let normal = if is_first {
                -manifold.normal
            } else {
                manifold.normal
            };

            // Solve each penetrating contact in the manifold
            let mut deepest_penetration: Scalar = Scalar::MIN;
            for contact in manifold.points.iter() {
                if contact.penetration > 0.0 {
                    position.0 += normal * contact.penetration;
                }
                deepest_penetration = deepest_penetration.max(contact.penetration);
            }

            // For now, this system only handles velocity corrections for collisions against
            // static geometry
            if is_other_dynamic {
                continue;
            }

            // Determine if the slope is climbable or too steep to walk on
            let slope_angle = normal.angle_between(Vector::Y);
            let climbable = slope_angle.abs() <= params.max_slope_angle;

            if deepest_penetration > 0.0 {
                // If the slope is climbable, snap velocity smoothly up and down the slope
                if climbable {
                    let normal_xz = normal.reject_from_normalized(Vector::Y).normalize_or_zero();
                    let velocity_xz = velocity.dot(normal_xz);

                    // Snap the Y speed based on the speed at which the character is moving up
                    // or down the slope, and how steep the slope is.
                    //
                    // A 2D visualization of the slope, the contact normal, and the velocity components:
                    //
                    //             ╱
                    //     normal ╱
                    // *         ╱
                    // │   *    ╱   velocity_x
                    // │       * - - - - - -
                    // │           *       | velocity_y
                    // │               *   |
                    // *───────────────────*

                    let max_y_speed = -velocity_xz * slope_angle.tan();
                    velocity.y = velocity.y.max(max_y_speed);
                } else {
                    // Slide along a non-climbable surface, similarly to a collide-and-slide
                    // algorithm

                    // Don't pply an impulse if the character is moving away from the surface
                    if velocity.dot(normal) > 0.0 {
                        continue;
                    }

                    // Slide along the surface (reject velocity along the contact normal)
                    let impulse = velocity.reject_from_normalized(normal);
                    velocity.0 = impulse;
                }
            } else {
                // The character is not yet intersecting the other object, but the narrow
                // phase detected a speculative collision.
                //
                // We need to push back the part of the velocity that would cause penetration
                // within the next frame.

                let normal_speed = velocity.dot(normal);

                // Don't apply an impulse if the character is moving away from the surface
                if normal_speed > 0.0 {
                    continue;
                }

                let impulse_magnitude = normal_speed - (deepest_penetration / time.delta_secs());
                let mut impulse = impulse_magnitude * normal;

                // Apply the impulse depending on the slope angle; avoid sliding down slopes or
                // climbing up walls
                if climbable {
                    velocity.y -= impulse.y.min(0.0);
                } else {
                    impulse.y = impulse.y.max(0.0);
                    velocity.0 -= impulse;
                }
            }
        }
    }
}
