use super::{
    Attachment, GROUNDING_PROXIMITY, HitProperties, PlayerController, PlayerHits, math,
    params::CharacterControllerParams,
};
use crate::{controller::Manoeuvrability, input::MovementState, state::AppState};
use avian3d::{
    math::{Quaternion, Vector},
    prelude::*,
};
use bevy::prelude::*;

// Known issues:
// - Moving aerially would benefit from different params for acceleration and deceleration

pub fn schedule_systems(app: &mut App) {
    app // Fixed timestep must exceed monitor refresh rate, else shape casts might not run, and problems happen
        .insert_resource(Time::<Fixed>::from_hz(96.0))
        .add_systems(
            PreUpdate,
            (
                apply_gravity, // Apply vertical accelerations: gravity, buoyancy, and terminal velocity
                apply_inputs,  // Apply horizontal accelerations and vertical jump impulses
            )
                .chain()
                .run_if(in_state(AppState::Game)),
        )
        .add_systems(
            PhysicsSchedule, // Run inside the FixedPostUpdate schedule; note FixedMain runs zero-to-many times before Update
            (
                move_player, // Apply physics on current velocity, updating position and velocity
                query_surrounding_hits, // Based on current position, query downward shape cast, and wall-facing shape cast if currently attached
            )
                .chain()
                .run_if(in_state(AppState::Game))
                .in_set(PhysicsStepSystems::Last),
        )
        .add_systems(
            Update,
            (
                update_markers, // Update player state markers according to current conditions
            )
                .run_if(in_state(AppState::Game)),
        );
}

const GROUND_CAST_MAX_HITS: u32 = 3;

struct MovementResult {
    new_position: Vec3,
    new_attachment: Option<Attachment>,
    new_velocity: Option<Vector>,
}

#[derive(PartialEq)]
enum CommonMarkerUpdates {
    Defaults,
    None,
}

#[derive(PartialEq)]
enum JumpMode {
    None,
    Regular,
}

enum ManoeuvreMode<'a> {
    HorizontalInput(&'a Manoeuvrability),
    PlanarInput {
        manoeuvrability: &'a Manoeuvrability,
        normal: Vector,
    },
}

fn query_surrounding_hits(
    mut player_query: Query<
        (
            Entity,
            &PlayerController,
            &mut PlayerHits,
            &mut Transform,
            &Rotation,
            Option<&Attachment>,
        ),
        With<PlayerController>,
    >,
    spatial_queries: Res<SpatialQueryPipeline>,
    params: Res<CharacterControllerParams>,
) -> Result<(), BevyError> {
    let Ok((entity, controller, mut hits, mut transform, rotation, attachment)) =
        player_query.single_mut()
    else {
        println!("Not running query_surrounding_hits this timestep");
        return Ok(());
    };

    // Cast from the player's position downwards
    let shape = PlayerController::collider(params.collider_skin_thickness);
    let shape_rotation = Quaternion::default();
    let direction = Dir3::NEG_Y;
    let config = ShapeCastConfig {
        max_distance: GROUNDING_PROXIMITY,
        ..default()
    };
    let filter = SpatialQueryFilter::default().with_excluded_entities([entity]);
    let shape_hits = spatial_queries.shape_hits(
        &shape,
        transform.translation,
        shape_rotation,
        direction,
        GROUND_CAST_MAX_HITS,
        &config,
        &filter,
    );

    // Record hits so long as the player is not moving somewhat away from the surface
    hits.ground = shape_hits
        .iter()
        .filter(|hit| hit.normal1.dot(controller.velocity) < params.escape_incidence)
        .next()
        .map(|hit| HitProperties::from_avian_hit(hit, rotation));

    match attachment {
        Some(Attachment::Grounded { .. }) => {
            if let Some(hit) = &hits.ground {
                transform.translation +=
                    (hit.distance - params.collider_skin_thickness) * direction.as_vec3();
            }
        }
        _ => {}
    }

    Ok(())
}

/// Update the [`Attachment`] and [`SpecialMove`] components for the [`PlayerController`]
fn update_markers(
    mut commands: Commands,
    mut player_query: Query<(Entity, &PlayerHits, Option<&Attachment>)>,
    params: Res<CharacterControllerParams>,
) -> Result<(), BevyError> {
    let Ok((entity, hits, attachment)) = player_query.single_mut() else {
        println!("Not running update_markers this timestep");
        return Ok(());
    };
    let grounding = match &hits.ground {
        Some(hit) if hit.normal_angle <= params.max_sliding_slope_angle => Some(hit.normal),
        _ => None,
    };
    let slope_is_walkable = match &hits.ground {
        Some(hit) if hit.normal_angle <= params.max_walking_slope_angle => true,
        _ => false,
    };

    let next_step: CommonMarkerUpdates = match attachment {
        None => {
            // Falling
            if let Some(normal) = grounding {
                if slope_is_walkable {
                    commands
                        .entity(entity)
                        .insert(Attachment::Grounded { normal });
                }
                CommonMarkerUpdates::None
            } else {
                CommonMarkerUpdates::Defaults
            }
        }
        Some(Attachment::Grounded { .. }) => {
            // Standing, Walking
            if grounding.is_none() {
                commands.entity(entity).remove::<Attachment>();
                CommonMarkerUpdates::None
            } else if !slope_is_walkable {
                commands.entity(entity).remove::<Attachment>();
                CommonMarkerUpdates::None
            } else {
                CommonMarkerUpdates::Defaults
            }
        }
    };

    Ok(())
}

fn apply_gravity(
    mut query: Query<(&mut PlayerController, Option<&Attachment>)>,
    params: Res<CharacterControllerParams>,
    time: Res<Time>,
) -> Result<(), BevyError> {
    let Ok((mut controller, attachment)) = query.single_mut() else {
        println!("Not running apply_gravity this timestep");
        return Ok(());
    };
    let delta_time = time.delta_secs();
    let (gravity, terminal_velocity) = match attachment {
        None => (params.gravity, params.terminal_velocity),
        Some(Attachment::Grounded { normal }) => {
            let is_walkable = normal.angle_between(Vector::Y) <= params.max_walking_slope_angle;
            match is_walkable {
                true => (Vector::ZERO, params.terminal_velocity),
                false => (params.gravity, params.terminal_velocity),
            }
        }
    };

    controller.velocity.y = math::approach_velocity(
        controller.velocity.y,
        gravity.y,
        delta_time,
        terminal_velocity,
    );

    Ok(())
}

fn apply_inputs(
    mut commands: Commands,
    mut query: Query<(Entity, &mut PlayerController, Option<&Attachment>)>,
    params: Res<CharacterControllerParams>,
    inputs: Res<MovementState>,
    time: Res<Time>,
) -> Result<(), BevyError> {
    let Ok((entity, mut controller, attachment)) = query.single_mut() else {
        println!("Not running apply_inputs this timestep");
        return Ok(());
    };
    let delta_time = time.delta_secs();
    let (jump_mode, manoevre_mode) = match attachment {
        Some(Attachment::Grounded { normal }) => {
            let normal_angle = normal.angle_between(Vector::Y);
            let input_mode = match normal_angle <= params.max_walking_slope_angle {
                true => ManoeuvreMode::PlanarInput {
                    manoeuvrability: &params.ground_movement,
                    normal: *normal,
                },
                false => ManoeuvreMode::HorizontalInput(&params.ground_movement),
            };
            (JumpMode::Regular, input_mode)
        }
        None => (
            JumpMode::None,
            ManoeuvreMode::HorizontalInput(&params.aerial_movement),
        ),
    };
    match manoevre_mode {
        ManoeuvreMode::HorizontalInput(factors) => {
            controller.velocity.x = match inputs.input_direction_x.abs() < std::f32::EPSILON {
                true => math::approach_zero(
                    controller.velocity.x,
                    delta_time,
                    factors.speed_factor * params.base_movement.speed_factor,
                    factors.stop_factor * params.base_movement.stop_factor,
                ),
                false => {
                    let input_direction = Vec3::new(inputs.input_direction_x, 0.0, 0.0);
                    let velocity_projection = controller.velocity.dot(input_direction);
                    let (base_factor, input_factor) = match velocity_projection.abs()
                        < std::f32::EPSILON
                        || controller.velocity.dot(input_direction) > 0.0
                    {
                        true => (params.base_movement.input_factor, factors.input_factor),
                        false => (
                            params.base_movement.reverse_input_factor,
                            factors.reverse_input_factor,
                        ),
                    };
                    math::approach_velocity(
                        controller.velocity.x,
                        input_factor * base_factor * inputs.input_direction_x,
                        delta_time,
                        factors.speed_factor
                            * params.base_movement.speed_factor
                            * inputs.input_direction_x,
                    )
                }
            };
        }
        ManoeuvreMode::PlanarInput {
            manoeuvrability: factors,
            normal,
        } => {
            match inputs.input_direction_x.abs() < std::f32::EPSILON {
                true => {
                    // No input, approach zero in current direction
                    let normal_velocity = controller.velocity.dot(normal) * normal;
                    let planar_velocity = controller.velocity - normal_velocity;
                    let speed = planar_velocity.length();
                    let new_speed = math::approach_zero(
                        speed,
                        delta_time,
                        factors.speed_factor * params.base_movement.speed_factor,
                        factors.stop_factor * params.base_movement.stop_factor,
                    );
                    controller.velocity = new_speed * planar_velocity.normalize_or_zero();
                }
                false => {
                    // Separate velocity in acceleration direction from remaining velocity, and
                    // accelerate just in that direction
                    let input_direction = Vec3::new(inputs.input_direction_x, 0.0, 0.0);
                    let velocity_projection = controller.velocity.dot(input_direction);
                    let (base_factor, input_factor) = match velocity_projection.abs()
                        < std::f32::EPSILON
                        || controller.velocity.dot(input_direction) > 0.0
                    {
                        true => (params.base_movement.input_factor, factors.input_factor),
                        false => (
                            params.base_movement.reverse_input_factor,
                            factors.reverse_input_factor,
                        ),
                    };
                    let planar_velocity = velocity_projection.abs() * input_direction;
                    let perpendicular_velocity = controller.velocity - planar_velocity;
                    let new_speed = math::approach_velocity(
                        planar_velocity.length(),
                        input_factor * base_factor * inputs.input_direction_x.abs(),
                        delta_time,
                        factors.speed_factor
                            * params.base_movement.speed_factor
                            * inputs.input_direction_x.abs(),
                    );
                    controller.velocity = perpendicular_velocity + new_speed * input_direction;
                }
            }
        }
    }
    if inputs.just_pressed_jump {
        match jump_mode {
            JumpMode::None => {}
            JumpMode::Regular => {
                controller.velocity.y += params.base_movement.jump_factor;
                commands.entity(entity).remove::<Attachment>();
            }
        }
    }
    controller.velocity.z = 0.0;
    Ok(())
}

/// Static bodies don't get updated by the physics engine, allowing a custom resolver to work.
fn move_player(
    mut commands: Commands,
    mut controllers_query: Query<(
        Entity,
        &mut Transform,
        &mut PlayerController,
        Option<&Attachment>,
    )>,
    spatial_queries: Res<SpatialQueryPipeline>,
    params: Res<CharacterControllerParams>,
    time: Res<Time>,
) -> Result<(), BevyError> {
    let Ok((entity, mut transform, mut controller, attachment)) = controllers_query.single_mut()
    else {
        println!("Not running move_player this timestep");
        return Ok(());
    };

    let shape = PlayerController::collider(params.collider_skin_thickness);
    let attempted_displacement = time.delta_secs() * controller.velocity;
    let is_grounded = match attachment {
        Some(Attachment::Grounded { .. }) => true,
        _ => false,
    };

    let result = move_and_collide_and_slide(
        spatial_queries,
        params,
        &shape,
        entity,
        transform.translation,
        attempted_displacement,
        0,
        controller.velocity.length(),
        is_grounded,
    );
    let travel = result.new_position - transform.translation;
    transform.translation = result.new_position;
    if let Some(attachment) = result.new_attachment {
        commands.entity(entity).insert(attachment);
    }
    controller.velocity = match result.new_velocity {
        Some(velocity) => velocity,
        None => travel / time.delta_secs(),
    };
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
    speed: f32,
    is_grounded: bool,
) -> MovementResult {
    let shape_rotation = Quaternion::default();
    let Ok(direction) = Dir3::new(attempted_displacement) else {
        return MovementResult {
            new_position: position + attempted_displacement,
            new_attachment: None,
            new_velocity: None,
        };
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
        None => MovementResult {
            new_position: position + attempted_displacement,
            new_attachment: None,
            new_velocity: None,
        },
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
                move_and_collide_and_slide(
                    spatial_queries,
                    params,
                    shape,
                    entity,
                    collision_position,
                    slide_displacement,
                    bounce_no + 1,
                    speed,
                    is_grounded,
                )
            } else {
                MovementResult {
                    new_position: collision_position,
                    new_attachment: None,
                    new_velocity: None,
                }
            }
        }
    }
}
