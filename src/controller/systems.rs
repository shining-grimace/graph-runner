use super::{
    Attachment, GROUNDING_PROXIMITY, HitProperties, MovementResult, PlayerAndWaterEntities,
    PlayerController, PlayerHits, SpecialMove, WALL_RETENTION_PROXIMITY,
    functions::check_aerial_hit_movement, math, params::CharacterControllerParams,
};
use crate::{
    controller::Manoeuvrability,
    game::PLAYER_HEIGHT,
    input::MovementState,
    markers::{WaterVolume, WaterVolumeExtents},
    state::AppState,
};
use avian3d::{
    math::{Quaternion, Scalar, Vector},
    prelude::*,
};
use bevy::{prelude::*, scene::SceneInstanceReady};

// Known issues:
// - The character still can intersect a static body (float on water, then push against a sloped bank)
// - Ledge-grabbing isn't implemented
// - Jumping with narrow clearance above a slopey peak can yank down to it

pub fn schedule_systems(app: &mut App) {
    app // Fixed timestep must exceed monitor refresh rate, else shape casts might not run, and problems happen
        .insert_resource(Time::<Fixed>::from_hz(96.0))
        .add_observer(on_scene_instance_ready)
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
                hack_position_to_transform,
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

#[derive(PartialEq)]
enum CommonMarkerUpdates {
    Advance,
    AdvanceRate(f32),
    None,
}

#[derive(PartialEq)]
enum JumpMode {
    None,
    Regular {
        factor: Scalar,
    },
    Dive {
        upward_factor: Scalar,
        horizontal_factor: Scalar,
    },
    Shallow {
        factor: Scalar,
    },
    AwayFromNormal {
        normal: Vec3,
        upward_impulse_factor: Scalar,
    },
    Climb {
        normal: Vec3,
    },
}

#[derive(PartialEq)]
enum SecondaryButtonMode {
    None,
    EnterRoll,
    KickFromWall { normal: Vector },
    StartStreaming { water_volume_entity: Entity },
}

enum ManoeuvreMode<'a> {
    Freeze,
    Freewheel,
    HorizontalInput(&'a Manoeuvrability),
    PlanarInput {
        manoeuvrability: &'a Manoeuvrability,
        normal: Vector,
    },
    RadialMovement(&'a Manoeuvrability),
}

fn on_scene_instance_ready(
    _on: On<SceneInstanceReady>,
    mut commands: Commands,
    player_query: Query<Entity, With<PlayerController>>,
    water_query: Query<Entity, With<WaterVolume>>,
) -> Result<(), BevyError> {
    let player_entity = player_query.single()?;
    let mut excluded_entities: Vec<Entity> = water_query.iter().collect();
    excluded_entities.push(player_entity);
    commands.insert_resource(PlayerAndWaterEntities::from_entities(&excluded_entities));
    Ok(())
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
    water_query: Query<(Entity, &GlobalTransform, &WaterVolumeExtents), Without<PlayerController>>,
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
    let internal_cast_distance = 0.5 * PLAYER_HEIGHT;
    let external_cast_distance = GROUNDING_PROXIMITY;
    let config = ShapeCastConfig {
        max_distance: internal_cast_distance + external_cast_distance,
        ..default()
    };
    let filter = SpatialQueryFilter::default().with_excluded_entities([entity]);
    let centre_cast_hits = spatial_queries.shape_hits(
        &shape,
        transform.translation + (internal_cast_distance * Vec3::Y),
        shape_rotation,
        direction,
        GROUND_CAST_MAX_HITS,
        &config,
        &filter,
    );

    // Record hits so long as the player is not moving somewhat away from the surface
    hits.ground = centre_cast_hits
        .iter()
        .filter(|hit| {
            !water_query
                .iter()
                .any(|(entity, _, _)| entity == hit.entity)
        })
        .filter(|hit| hit.distance >= internal_cast_distance)
        .filter(|hit| hit.normal1.dot(controller.velocity) < params.escape_incidence)
        .next()
        .map(|hit| HitProperties::from_avian_hit(hit, rotation, internal_cast_distance));
    hits.water_surface = centre_cast_hits
        .iter()
        .filter(|hit| {
            water_query
                .iter()
                .any(|(entity, _, _)| entity == hit.entity)
        })
        .next()
        .map(|hit| HitProperties::from_avian_hit(hit, rotation, 0.0));
    hits.water_volume = water_query
        .iter()
        .filter(|(_, water_transform, water_volume)| {
            let water_translation = water_transform.translation();
            if water_transform.translation() == Vec3::ZERO {
                panic!("A water volume has a zero origin. This is probably not what you want.");
            }
            math::inside_volume(
                &transform.translation,
                &water_translation,
                water_volume,
                params.collider_skin_thickness,
            )
        })
        .next()
        .map(|(entity, _, _)| entity);

    // Some further updates: assign wall hit, and snap to surfaces
    match attachment {
        Some(Attachment::Grounded { .. }) => {
            if let Some(hit) = &hits.ground {
                transform.translation +=
                    (hit.distance - params.collider_skin_thickness) * direction.as_vec3();
            }
            hits.wall = None;
        }
        Some(Attachment::Walled { normal, .. }) => {
            let direction = Dir3::new(-normal)?;
            let config = ShapeCastConfig {
                max_distance: WALL_RETENTION_PROXIMITY,
                ..default()
            };
            let shape_hit = spatial_queries.cast_shape(
                &shape,
                transform.translation,
                shape_rotation,
                direction,
                &config,
                &filter,
            );
            hits.wall = shape_hit.map(|hit| HitProperties::from_avian_hit(&hit, rotation, 0.0));
        }
        Some(Attachment::Floating { .. }) => {
            if let Some(hit) = &hits.water_surface {
                transform.translation +=
                    (hit.distance - params.collider_skin_thickness) * direction.as_vec3();
            }
        }
        _ => {
            hits.wall = None;
        }
    }

    Ok(())
}

/// Hack: Replace Position with the Transform's translation just before the [`PhysicsTransformPlugin`]
/// writes Position back to the Transform.
/// Not sure how to control this properly so that the writeback doesn't occur for the player entity.
/// I wonder if [`ApplyPosToTransform`] can be not applied to the player?
fn hack_position_to_transform(
    mut player_query: Query<(&Transform, &mut Position), With<PlayerController>>,
) -> Result<(), BevyError> {
    let Ok((transform, mut position)) = player_query.single_mut() else {
        println!("Not running hack_position_to_transform this timestep");
        return Ok(());
    };
    position.0 = transform.translation;
    Ok(())
}

/// Update the [`Attachment`] and [`SpecialMove`] components for the [`PlayerController`]
fn update_markers(
    mut commands: Commands,
    mut player_query: Query<(
        Entity,
        &PlayerHits,
        &PlayerController,
        &mut Transform,
        Option<&Attachment>,
        Option<&SpecialMove>,
    )>,
    inputs: Res<MovementState>,
    params: Res<CharacterControllerParams>,
    time: Res<Time>,
) -> Result<(), BevyError> {
    let Ok((entity, hits, controller, mut transform, attachment, special_move)) =
        player_query.single_mut()
    else {
        println!("Not running update_markers this timestep");
        return Ok(());
    };
    let vertical_speed = controller.velocity.dot(Vector::Y);
    let horizontal_speed = controller.velocity.dot(
        controller
            .velocity
            .with_y(0.0)
            .try_normalize()
            .unwrap_or(Vector::X),
    );
    let input_opposes_motion = inputs.input_direction_x != 0.0
        && controller
            .velocity
            .dot(Vec3::new(inputs.input_direction_x, 0.0, 0.0))
            < 0.0;
    let grounding = match &hits.ground {
        Some(hit) if hit.normal_angle <= params.max_sliding_slope_angle => Some(hit.normal),
        _ => None,
    };
    let slope_is_walkable = match &hits.ground {
        Some(hit) if hit.normal_angle <= params.max_walking_slope_angle => true,
        _ => false,
    };
    let above_running_speed = horizontal_speed > params.running_speed;
    let is_landing_hard =
        grounding.is_some() && vertical_speed < -params.landing_stall_speed_threshold;
    let is_landing_fast =
        grounding.is_some() && horizontal_speed > params.landing_roll_speed_threshold;
    let is_on_wall = hits.wall.is_some();
    let on_water_surface_entity = hits.water_surface.as_ref().map(|hit| hit.entity);
    let in_water_volume = hits.water_volume;

    let next_step: CommonMarkerUpdates = match attachment {
        None => match special_move {
            None => {
                // Falling
                if let Some(normal) = grounding {
                    if !slope_is_walkable {
                        commands
                            .entity(entity)
                            .insert((Attachment::Grounded { normal }, SpecialMove::Sliding));
                    } else if is_landing_fast {
                        commands.entity(entity).insert((
                            Attachment::Grounded { normal },
                            SpecialMove::Rolling { progress: 0.0 },
                        ));
                    } else if is_landing_hard {
                        commands.entity(entity).insert((
                            Attachment::Grounded { normal },
                            SpecialMove::Landing { progress: 0.0 },
                        ));
                    } else {
                        commands
                            .entity(entity)
                            .insert(Attachment::Grounded { normal });
                    }
                    CommonMarkerUpdates::None
                } else if let Some(water_volume_entity) = in_water_volume {
                    commands.entity(entity).insert((
                        Attachment::Submerged {
                            water_volume_entity,
                        },
                        SpecialMove::Halting { progress: 0.0 },
                    ));
                    CommonMarkerUpdates::None
                } else {
                    CommonMarkerUpdates::Advance
                }
            }
            Some(SpecialMove::Landing { .. }) => CommonMarkerUpdates::Advance, // Should not happen
            Some(SpecialMove::Rolling { .. }) => {
                // Rolled off of ground
                if horizontal_speed < params.unroll_speed_threshold {
                    commands.entity(entity).remove::<SpecialMove>();
                    CommonMarkerUpdates::None
                } else {
                    // Unroll after extended delay while in the air
                    CommonMarkerUpdates::AdvanceRate(params.aerial_roll_progress_rate)
                }
            }
            Some(SpecialMove::Running) => CommonMarkerUpdates::Advance, // Should not happen
            Some(SpecialMove::Halting { .. }) => CommonMarkerUpdates::Advance, // Should not happen
            Some(SpecialMove::Sliding) => {
                // Falling while in a slide move
                if let Some(normal) = grounding {
                    commands
                        .entity(entity)
                        .insert(Attachment::Grounded { normal });
                    CommonMarkerUpdates::None
                } else if let Some(water_volume_entity) = in_water_volume {
                    commands.entity(entity).insert((
                        Attachment::Submerged {
                            water_volume_entity,
                        },
                        SpecialMove::Halting { progress: 0.0 },
                    ));
                    CommonMarkerUpdates::None
                } else {
                    CommonMarkerUpdates::Advance
                }
            }
            Some(SpecialMove::Jumping { .. }) => {
                // Jumping upward
                if let Some(normal) = grounding {
                    if !slope_is_walkable {
                        commands
                            .entity(entity)
                            .insert((Attachment::Grounded { normal }, SpecialMove::Sliding));
                    } else if is_landing_fast {
                        commands.entity(entity).insert((
                            Attachment::Grounded { normal },
                            SpecialMove::Rolling { progress: 0.0 },
                        ));
                    } else if is_landing_hard {
                        commands.entity(entity).insert((
                            Attachment::Grounded { normal },
                            SpecialMove::Landing { progress: 0.0 },
                        ));
                    } else {
                        commands
                            .entity(entity)
                            .insert(Attachment::Grounded { normal });
                    }
                    CommonMarkerUpdates::None
                } else if let Some(water_volume_entity) = in_water_volume {
                    commands.entity(entity).insert((
                        Attachment::Submerged {
                            water_volume_entity,
                        },
                        SpecialMove::Halting { progress: 0.0 },
                    ));
                    CommonMarkerUpdates::None
                } else if !inputs.pressing_jump || vertical_speed < 0.0 {
                    commands.entity(entity).remove::<SpecialMove>();
                    CommonMarkerUpdates::None
                } else {
                    CommonMarkerUpdates::Advance
                }
            }
            Some(SpecialMove::Diving) => {
                // Diving through air
                if let Some(normal) = grounding {
                    if !slope_is_walkable {
                        commands
                            .entity(entity)
                            .insert((Attachment::Grounded { normal }, SpecialMove::Sliding));
                    } else if is_landing_fast {
                        commands.entity(entity).insert((
                            Attachment::Grounded { normal },
                            SpecialMove::Rolling { progress: 0.0 },
                        ));
                    } else if is_landing_hard {
                        commands.entity(entity).insert((
                            Attachment::Grounded { normal },
                            SpecialMove::Landing { progress: 0.0 },
                        ));
                    } else {
                        commands
                            .entity(entity)
                            .insert(Attachment::Grounded { normal });
                        commands.entity(entity).remove::<SpecialMove>();
                    }
                    CommonMarkerUpdates::None
                } else if let Some(water_volume_entity) = in_water_volume {
                    commands.entity(entity).insert((
                        Attachment::Submerged {
                            water_volume_entity,
                        },
                        SpecialMove::Halting { progress: 0.0 },
                    ));
                    CommonMarkerUpdates::None
                } else {
                    CommonMarkerUpdates::Advance
                }
            }
            Some(SpecialMove::Climbing { .. }) => CommonMarkerUpdates::Advance, // Should not happen
        },
        Some(Attachment::Grounded { .. }) => match special_move {
            None => {
                // Standing, Walking
                if let Some(water_volume_entity) = in_water_volume {
                    commands.entity(entity).insert((
                        Attachment::Submerged {
                            water_volume_entity,
                        },
                        SpecialMove::Halting { progress: 0.0 },
                    ));
                    CommonMarkerUpdates::None
                } else if grounding.is_none() {
                    commands.entity(entity).remove::<Attachment>();
                    CommonMarkerUpdates::None
                } else if !slope_is_walkable {
                    commands.entity(entity).insert(SpecialMove::Sliding);
                    CommonMarkerUpdates::None
                } else if above_running_speed {
                    commands.entity(entity).insert(SpecialMove::Running);
                    CommonMarkerUpdates::None
                } else {
                    CommonMarkerUpdates::Advance
                }
            }
            Some(SpecialMove::Landing { .. }) => {
                // Stationary after landing hard
                if let Some(water_volume_entity) = in_water_volume {
                    commands.entity(entity).insert((
                        Attachment::Submerged {
                            water_volume_entity,
                        },
                        SpecialMove::Halting { progress: 0.0 },
                    ));
                    CommonMarkerUpdates::None
                } else if grounding.is_none() {
                    commands.entity(entity).remove::<Attachment>();
                    CommonMarkerUpdates::None
                } else if !slope_is_walkable {
                    commands.entity(entity).insert(SpecialMove::Sliding);
                    CommonMarkerUpdates::None
                } else {
                    CommonMarkerUpdates::Advance
                }
            }
            Some(SpecialMove::Rolling { .. }) => {
                // Landed fast, or pressed secondary button while running
                if let Some(water_volume_entity) = in_water_volume {
                    commands.entity(entity).insert((
                        Attachment::Submerged {
                            water_volume_entity,
                        },
                        SpecialMove::Halting { progress: 0.0 },
                    ));
                    CommonMarkerUpdates::None
                } else if grounding.is_none() {
                    commands.entity(entity).remove::<Attachment>();
                    CommonMarkerUpdates::None
                } else if horizontal_speed < params.unroll_speed_threshold {
                    commands.entity(entity).remove::<SpecialMove>();
                    CommonMarkerUpdates::None
                } else {
                    CommonMarkerUpdates::Advance
                }
            }
            Some(SpecialMove::Running) => {
                // Running on ground
                if let Some(water_volume_entity) = in_water_volume {
                    commands.entity(entity).insert((
                        Attachment::Submerged {
                            water_volume_entity,
                        },
                        SpecialMove::Halting { progress: 0.0 },
                    ));
                    CommonMarkerUpdates::None
                } else if grounding.is_none() {
                    commands.entity(entity).remove::<Attachment>();
                    commands.entity(entity).remove::<SpecialMove>();
                    CommonMarkerUpdates::None
                } else if !slope_is_walkable {
                    commands.entity(entity).insert(SpecialMove::Sliding);
                    CommonMarkerUpdates::None
                } else if !above_running_speed {
                    commands.entity(entity).remove::<SpecialMove>();
                    CommonMarkerUpdates::None
                } else if input_opposes_motion {
                    commands
                        .entity(entity)
                        .insert(SpecialMove::Halting { progress: 0.0 });
                    CommonMarkerUpdates::None
                } else {
                    CommonMarkerUpdates::Advance
                }
            }
            Some(SpecialMove::Halting { .. }) => {
                // Was running and pushed hard in the opposite direction
                if let Some(water_volume_entity) = in_water_volume {
                    commands.entity(entity).insert((
                        Attachment::Submerged {
                            water_volume_entity,
                        },
                        SpecialMove::Halting { progress: 0.0 },
                    ));
                    CommonMarkerUpdates::None
                } else if grounding.is_none() {
                    commands.entity(entity).remove::<Attachment>();
                    CommonMarkerUpdates::None
                } else if !slope_is_walkable {
                    commands.entity(entity).insert(SpecialMove::Sliding);
                    CommonMarkerUpdates::None
                } else {
                    CommonMarkerUpdates::Advance
                }
            }
            Some(SpecialMove::Sliding) => {
                // Was on a steep slope (keep sliding even if not anymore)
                if let Some(water_volume_entity) = in_water_volume {
                    commands.entity(entity).insert((
                        Attachment::Submerged {
                            water_volume_entity,
                        },
                        SpecialMove::Halting { progress: 0.0 },
                    ));
                    CommonMarkerUpdates::None
                } else if grounding.is_none() {
                    commands.entity(entity).remove::<Attachment>();
                    commands.entity(entity).remove::<SpecialMove>();
                    CommonMarkerUpdates::None
                } else if horizontal_speed < params.unslide_speed_threshold && slope_is_walkable {
                    commands.entity(entity).remove::<SpecialMove>();
                    CommonMarkerUpdates::None
                } else {
                    CommonMarkerUpdates::Advance
                }
            }
            Some(SpecialMove::Jumping) => CommonMarkerUpdates::Advance, // Should not happen
            Some(SpecialMove::Diving) => CommonMarkerUpdates::Advance,  // Should not happen
            Some(SpecialMove::Climbing { .. }) => CommonMarkerUpdates::Advance, // Should not happen
        },
        Some(Attachment::LedgeGrabbed { .. }) => match special_move {
            None => {
                // Holding onto a ledge
                if let Some(water_volume_entity) = in_water_volume {
                    commands.entity(entity).insert((
                        Attachment::Submerged {
                            water_volume_entity,
                        },
                        SpecialMove::Halting { progress: 0.0 },
                    ));
                    CommonMarkerUpdates::None
                } else {
                    CommonMarkerUpdates::Advance
                }
            }
            Some(SpecialMove::Landing { .. }) => CommonMarkerUpdates::Advance, // Should not happen
            Some(SpecialMove::Rolling { .. }) => CommonMarkerUpdates::Advance, // Should not happen
            Some(SpecialMove::Running) => CommonMarkerUpdates::Advance,        // Should not happen
            Some(SpecialMove::Halting { .. }) => {
                // Just became attached to the ledge; no control briefly
                if let Some(water_volume_entity) = in_water_volume {
                    commands.entity(entity).insert((
                        Attachment::Submerged {
                            water_volume_entity,
                        },
                        SpecialMove::Halting { progress: 0.0 },
                    ));
                    CommonMarkerUpdates::None
                } else {
                    CommonMarkerUpdates::Advance
                }
            }
            Some(SpecialMove::Sliding) => CommonMarkerUpdates::Advance, // Should not happen
            Some(SpecialMove::Jumping) => CommonMarkerUpdates::Advance, // Should not happen
            Some(SpecialMove::Diving) => CommonMarkerUpdates::Advance,  // Should not happen
            Some(SpecialMove::Climbing { .. }) => CommonMarkerUpdates::Advance,
        },
        Some(Attachment::Walled { .. }) => match special_move {
            None => {
                // Stuck to a wall for a length of time before starting to slip
                if let Some(water_volume_entity) = in_water_volume {
                    commands.entity(entity).insert((
                        Attachment::Submerged {
                            water_volume_entity,
                        },
                        SpecialMove::Halting { progress: 0.0 },
                    ));
                    CommonMarkerUpdates::None
                } else if !is_on_wall {
                    commands.entity(entity).remove::<Attachment>();
                    CommonMarkerUpdates::None
                } else {
                    CommonMarkerUpdates::Advance
                }
            }
            Some(SpecialMove::Landing { .. }) => CommonMarkerUpdates::Advance, // Should not happen
            Some(SpecialMove::Rolling { .. }) => CommonMarkerUpdates::Advance, // Should not happen
            Some(SpecialMove::Running) => CommonMarkerUpdates::Advance,        // Should not happen
            Some(SpecialMove::Halting { .. }) => CommonMarkerUpdates::Advance, // Should not happen
            Some(SpecialMove::Sliding) => {
                // Sliding down wall
                if let Some(water_volume_entity) = in_water_volume {
                    commands.entity(entity).insert((
                        Attachment::Submerged {
                            water_volume_entity,
                        },
                        SpecialMove::Halting { progress: 0.0 },
                    ));
                    CommonMarkerUpdates::None
                } else if let Some(normal) = grounding {
                    if slope_is_walkable {
                        commands
                            .entity(entity)
                            .insert(Attachment::Grounded { normal });
                    } else {
                        commands
                            .entity(entity)
                            .insert((Attachment::Grounded { normal }, SpecialMove::Sliding));
                    }
                    CommonMarkerUpdates::None
                } else if !is_on_wall {
                    commands.entity(entity).remove::<Attachment>();
                    CommonMarkerUpdates::None
                } else {
                    CommonMarkerUpdates::Advance
                }
            }
            Some(SpecialMove::Jumping) => CommonMarkerUpdates::Advance, // Should not happen
            Some(SpecialMove::Diving) => CommonMarkerUpdates::Advance,  // Should not happen
            Some(SpecialMove::Climbing { .. }) => CommonMarkerUpdates::Advance, // Should not happen
        },
        Some(Attachment::Submerged { .. }) => match special_move {
            None => {
                // Swimming
                if in_water_volume.is_none() {
                    if let Some(water_entity) = on_water_surface_entity {
                        commands.entity(entity).insert(Attachment::Floating {
                            water_volume_entity: water_entity,
                        });
                        commands.entity(entity).remove::<SpecialMove>();
                        CommonMarkerUpdates::None
                    } else {
                        commands.entity(entity).remove::<Attachment>();
                        CommonMarkerUpdates::None
                    }
                } else {
                    CommonMarkerUpdates::Advance
                }
            }
            Some(SpecialMove::Landing { .. }) => CommonMarkerUpdates::Advance, // Should not happen
            Some(SpecialMove::Rolling { .. }) => CommonMarkerUpdates::Advance, // Should not happen
            Some(SpecialMove::Running) => CommonMarkerUpdates::Advance,        // Should not happen
            Some(SpecialMove::Halting { .. }) => {
                // Just became submerged, no control briefly
                if in_water_volume.is_none() {
                    commands.entity(entity).remove::<Attachment>();
                    CommonMarkerUpdates::None
                } else {
                    CommonMarkerUpdates::Advance
                }
            }
            Some(SpecialMove::Sliding) => CommonMarkerUpdates::Advance, // Should not happen
            Some(SpecialMove::Jumping) => CommonMarkerUpdates::Advance, // Should not happen
            Some(SpecialMove::Diving) => {
                // Streaming through water
                if in_water_volume.is_none() {
                    commands.entity(entity).remove::<Attachment>();
                    CommonMarkerUpdates::None
                } else if vertical_speed == 0.0 && horizontal_speed == 0.0 {
                    commands.entity(entity).remove::<SpecialMove>();
                    CommonMarkerUpdates::None
                } else {
                    CommonMarkerUpdates::Advance
                }
            }
            Some(SpecialMove::Climbing { .. }) => CommonMarkerUpdates::Advance, // Should not happen
        },
        Some(Attachment::Floating { .. }) => match special_move {
            None => {
                // Paddling around on the surface
                if on_water_surface_entity.is_none() {
                    commands.entity(entity).remove::<Attachment>();
                    CommonMarkerUpdates::None
                } else {
                    CommonMarkerUpdates::Advance
                }
            }
            Some(SpecialMove::Landing { .. }) => CommonMarkerUpdates::Advance, // Should not happen
            Some(SpecialMove::Rolling { .. }) => CommonMarkerUpdates::Advance, // Should not happen
            Some(SpecialMove::Running) => CommonMarkerUpdates::Advance,        // Should not happen
            Some(SpecialMove::Halting { .. }) => CommonMarkerUpdates::Advance, // Should not happen
            Some(SpecialMove::Sliding) => CommonMarkerUpdates::Advance,        // Should not happen
            Some(SpecialMove::Jumping) => CommonMarkerUpdates::Advance,        // Should not happen
            Some(SpecialMove::Diving) => CommonMarkerUpdates::Advance,         // Should not happen
            Some(SpecialMove::Climbing { .. }) => CommonMarkerUpdates::Advance, // Should not happen
        },
    };

    // If nothing changed, handle delays on the special move
    let advance_rate = match next_step {
        CommonMarkerUpdates::Advance => 1.0,
        CommonMarkerUpdates::AdvanceRate(rate) => rate,
        CommonMarkerUpdates::None => {
            return Ok(());
        }
    };
    match attachment {
        Some(Attachment::Walled { normal, progress }) => {
            if special_move.is_none() {
                let progress = progress + advance_rate * time.delta_secs();
                if progress > params.wall_stick_duration {
                    commands.entity(entity).insert(SpecialMove::Sliding);
                } else {
                    commands.entity(entity).insert(Attachment::Walled {
                        normal: *normal,
                        progress,
                    });
                }
                return Ok(());
            }
        }
        _ => {}
    }
    match special_move {
        None => {}
        Some(SpecialMove::Landing { progress }) => {
            let progress = progress + advance_rate * time.delta_secs();
            if progress > params.landing_move_duration {
                commands.entity(entity).remove::<SpecialMove>();
            } else {
                commands
                    .entity(entity)
                    .insert(SpecialMove::Landing { progress });
            }
        }
        Some(SpecialMove::Rolling { progress }) => {
            let progress = progress + advance_rate * time.delta_secs();
            if progress > params.rolling_move_duration {
                commands.entity(entity).remove::<SpecialMove>();
            } else {
                commands
                    .entity(entity)
                    .insert(SpecialMove::Rolling { progress });
            }
        }
        Some(SpecialMove::Running) => {}
        Some(SpecialMove::Halting { progress }) => {
            let duration = match attachment {
                Some(Attachment::Submerged { .. }) => params.submersion_move_duration,
                _ => params.halting_move_duration,
            };
            let progress = progress + advance_rate * time.delta_secs();
            if progress > duration {
                commands.entity(entity).remove::<SpecialMove>();
            } else {
                commands
                    .entity(entity)
                    .insert(SpecialMove::Halting { progress });
            }
        }
        Some(SpecialMove::Sliding) => {}
        Some(SpecialMove::Jumping) => {}
        Some(SpecialMove::Diving) => {}
        Some(SpecialMove::Climbing { progress, normal }) => {
            let progress = progress + advance_rate * time.delta_secs();
            if progress > params.climbing_move_duration {
                commands
                    .entity(entity)
                    .remove::<Attachment>()
                    .remove::<SpecialMove>();
                transform.translation -= params.ledge_grab_required_inset * normal;
            } else {
                commands.entity(entity).insert(SpecialMove::Climbing {
                    progress,
                    normal: *normal,
                });
            }
        }
    }

    Ok(())
}

fn apply_gravity(
    mut query: Query<(
        &mut PlayerController,
        Option<&Attachment>,
        Option<&SpecialMove>,
    )>,
    params: Res<CharacterControllerParams>,
    time: Res<Time>,
) -> Result<(), BevyError> {
    let Ok((mut controller, attachment, special_move)) = query.single_mut() else {
        println!("Not running apply_gravity this timestep");
        return Ok(());
    };
    let delta_time = time.delta_secs();
    let (gravity, terminal_velocity) = match attachment {
        None => match special_move {
            Some(SpecialMove::Jumping) => (
                params.jumping_gravity_factor * params.gravity,
                params.terminal_velocity,
            ),
            Some(SpecialMove::Rolling { .. }) => (
                params.aerial_roll_gravity_factor * params.gravity,
                params.terminal_velocity,
            ),
            _ => (params.gravity, params.terminal_velocity),
        },
        Some(Attachment::Grounded { normal }) => {
            let is_walkable = normal.angle_between(Vector::Y) <= params.max_walking_slope_angle;
            match is_walkable {
                true => (Vector::ZERO, params.terminal_velocity),
                false => (params.gravity, params.terminal_velocity),
            }
        }
        Some(Attachment::LedgeGrabbed { .. }) => (Vector::ZERO, 0.0),
        Some(Attachment::Walled { .. }) => match special_move {
            Some(SpecialMove::Sliding) => (params.gravity, params.wall_slide_terminal_velocity),
            _ => (Vector::ZERO, 0.0),
        },
        Some(Attachment::Submerged { .. }) => match special_move {
            Some(SpecialMove::Diving) => (Vector::ZERO, params.terminal_velocity),
            _ => (params.buoyant_gravity, params.buoyant_terminal_velocity),
        },
        Some(Attachment::Floating { .. }) => (params.gravity, params.terminal_velocity),
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
    mut query: Query<(
        Entity,
        &mut PlayerController,
        Option<&Attachment>,
        Option<&SpecialMove>,
    )>,
    params: Res<CharacterControllerParams>,
    inputs: Res<MovementState>,
    time: Res<Time>,
) -> Result<(), BevyError> {
    let Ok((entity, mut controller, attachment, special_move)) = query.single_mut() else {
        println!("Not running apply_inputs this timestep");
        return Ok(());
    };
    let delta_time = time.delta_secs();
    let (jump_mode, secondary_button_mode, manoevre_mode) = match attachment {
        Some(Attachment::Grounded { normal }) => match special_move {
            Some(SpecialMove::Landing { .. }) => (
                JumpMode::None,
                SecondaryButtonMode::None,
                ManoeuvreMode::Freeze,
            ),
            Some(SpecialMove::Rolling { .. }) => (
                JumpMode::Shallow {
                    factor: params.rolling_movement.jump_factor,
                },
                SecondaryButtonMode::None,
                ManoeuvreMode::HorizontalInput(&params.rolling_movement),
            ),
            Some(SpecialMove::Running) => (
                JumpMode::Regular {
                    factor: params.ground_movement.jump_factor,
                },
                SecondaryButtonMode::EnterRoll,
                ManoeuvreMode::HorizontalInput(&params.ground_movement),
            ),
            Some(SpecialMove::Halting { .. }) => (
                JumpMode::Dive {
                    upward_factor: params.dive_jump_upward_factor,
                    horizontal_factor: params.dive_jump_horizontal_factor,
                },
                SecondaryButtonMode::None,
                ManoeuvreMode::HorizontalInput(&params.halting_movement),
            ),
            Some(SpecialMove::Sliding) => (
                JumpMode::Shallow {
                    factor: params.sliding_movement.jump_factor,
                },
                SecondaryButtonMode::None,
                ManoeuvreMode::HorizontalInput(&params.sliding_movement),
            ),
            Some(SpecialMove::Jumping) => (
                JumpMode::None,
                SecondaryButtonMode::None,
                ManoeuvreMode::Freewheel,
            ),
            Some(SpecialMove::Diving) => (
                JumpMode::None,
                SecondaryButtonMode::None,
                ManoeuvreMode::Freewheel,
            ),
            Some(SpecialMove::Climbing { .. }) => (
                JumpMode::None,
                SecondaryButtonMode::None,
                ManoeuvreMode::Freewheel,
            ),
            None => {
                let normal_angle = normal.angle_between(Vector::Y);
                let input_mode = match normal_angle <= params.max_walking_slope_angle {
                    true => ManoeuvreMode::PlanarInput {
                        manoeuvrability: &params.ground_movement,
                        normal: *normal,
                    },
                    false => ManoeuvreMode::HorizontalInput(&params.ground_movement),
                };
                (
                    JumpMode::Regular {
                        factor: params.ground_movement.jump_factor,
                    },
                    SecondaryButtonMode::None,
                    input_mode,
                )
            }
        },
        Some(Attachment::LedgeGrabbed { normal, .. }) => (
            JumpMode::Climb { normal: *normal },
            SecondaryButtonMode::KickFromWall { normal: *normal },
            ManoeuvreMode::Freeze,
        ),
        Some(Attachment::Walled { normal, .. }) => match special_move {
            Some(SpecialMove::Landing { .. }) => (
                JumpMode::None,
                SecondaryButtonMode::None,
                ManoeuvreMode::Freewheel,
            ),
            Some(SpecialMove::Rolling { .. }) => (
                JumpMode::None,
                SecondaryButtonMode::None,
                ManoeuvreMode::Freewheel,
            ),
            Some(SpecialMove::Running) => (
                JumpMode::None,
                SecondaryButtonMode::None,
                ManoeuvreMode::Freewheel,
            ),
            Some(SpecialMove::Halting { .. }) => (
                JumpMode::None,
                SecondaryButtonMode::None,
                ManoeuvreMode::Freewheel,
            ),
            Some(SpecialMove::Sliding) => (
                JumpMode::AwayFromNormal {
                    normal: *normal,
                    upward_impulse_factor: params.shallow_wall_jump_upward_factor,
                },
                SecondaryButtonMode::KickFromWall { normal: *normal },
                ManoeuvreMode::Freewheel,
            ),
            Some(SpecialMove::Jumping) => (
                JumpMode::None,
                SecondaryButtonMode::None,
                ManoeuvreMode::Freewheel,
            ),
            Some(SpecialMove::Diving) => (
                JumpMode::None,
                SecondaryButtonMode::None,
                ManoeuvreMode::Freewheel,
            ),
            Some(SpecialMove::Climbing { .. }) => (
                JumpMode::None,
                SecondaryButtonMode::None,
                ManoeuvreMode::Freewheel,
            ),
            None => (
                JumpMode::AwayFromNormal {
                    normal: *normal,
                    upward_impulse_factor: params.ground_movement.jump_factor,
                },
                SecondaryButtonMode::KickFromWall { normal: *normal },
                ManoeuvreMode::Freeze,
            ),
        },
        Some(Attachment::Submerged {
            water_volume_entity,
        }) => match special_move {
            Some(SpecialMove::Landing { .. }) => (
                JumpMode::None,
                SecondaryButtonMode::None,
                ManoeuvreMode::HorizontalInput(&params.submerged_movement),
            ),
            Some(SpecialMove::Rolling { .. }) => (
                JumpMode::None,
                SecondaryButtonMode::None,
                ManoeuvreMode::HorizontalInput(&params.submerged_movement),
            ),
            Some(SpecialMove::Running) => (
                JumpMode::None,
                SecondaryButtonMode::None,
                ManoeuvreMode::HorizontalInput(&params.submerged_movement),
            ),
            Some(SpecialMove::Halting { .. }) => (
                JumpMode::None,
                SecondaryButtonMode::None,
                ManoeuvreMode::HorizontalInput(&params.submerged_movement),
            ),
            Some(SpecialMove::Sliding) => (
                JumpMode::None,
                SecondaryButtonMode::None,
                ManoeuvreMode::HorizontalInput(&params.submerged_movement),
            ),
            Some(SpecialMove::Jumping) => (
                JumpMode::None,
                SecondaryButtonMode::None,
                ManoeuvreMode::HorizontalInput(&params.submerged_movement),
            ),
            Some(SpecialMove::Diving) => (
                JumpMode::None,
                SecondaryButtonMode::None,
                ManoeuvreMode::RadialMovement(&params.submerged_movement),
            ),
            Some(SpecialMove::Climbing { .. }) => (
                JumpMode::None,
                SecondaryButtonMode::None,
                ManoeuvreMode::HorizontalInput(&params.submerged_movement),
            ),
            None => (
                JumpMode::None,
                SecondaryButtonMode::StartStreaming {
                    water_volume_entity: *water_volume_entity,
                },
                ManoeuvreMode::HorizontalInput(&params.submerged_movement),
            ),
        },
        Some(Attachment::Floating {
            water_volume_entity,
        }) => (
            JumpMode::Shallow {
                factor: params.floating_movement.jump_factor,
            },
            SecondaryButtonMode::StartStreaming {
                water_volume_entity: *water_volume_entity,
            },
            ManoeuvreMode::PlanarInput {
                manoeuvrability: &params.floating_movement,
                normal: Vec3::Y,
            },
        ),
        None => (
            JumpMode::None,
            SecondaryButtonMode::None,
            ManoeuvreMode::HorizontalInput(&params.aerial_movement),
        ),
    };
    match manoevre_mode {
        ManoeuvreMode::Freeze => {
            controller.velocity.x = 0.0;
        }
        ManoeuvreMode::Freewheel => {}
        ManoeuvreMode::HorizontalInput(factors) => {
            controller.velocity.x = match inputs.input_direction_x.abs() < std::f32::EPSILON {
                true => math::approach_zero(
                    controller.velocity.x,
                    delta_time,
                    factors.max_speed_factor * params.base_movement.max_speed_factor,
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
                        factors.max_speed_factor
                            * params.base_movement.max_speed_factor
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
                        factors.max_speed_factor * params.base_movement.max_speed_factor,
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
                        factors.max_speed_factor
                            * params.base_movement.max_speed_factor
                            * inputs.input_direction_x.abs(),
                    );
                    controller.velocity = perpendicular_velocity + new_speed * input_direction;
                }
            }
        }
        ManoeuvreMode::RadialMovement(factors) => {
            let input_vector = Vec3::new(inputs.input_direction_x, inputs.input_direction_y, 0.0)
                .normalize_or_zero();
            let current_speed = controller.velocity.length();
            let approach_speed = input_vector.length();
            if approach_speed == 0.0 {
                let new_speed = math::approach_zero(
                    current_speed,
                    delta_time,
                    factors.max_speed_factor * params.base_movement.max_speed_factor,
                    factors.stop_factor * params.base_movement.stop_factor,
                );
                controller.velocity = new_speed * controller.velocity.normalize_or_zero();
            } else {
                let new_speed = math::approach_velocity(
                    current_speed,
                    factors.input_factor * params.base_movement.input_factor,
                    delta_time,
                    factors.max_speed_factor * params.base_movement.max_speed_factor,
                );
                let current_direction = controller.velocity.y.atan2(controller.velocity.x);
                let approach_direction = input_vector.y.atan2(input_vector.x);
                let relative_approach_direction = math::float_modulus(
                    approach_direction - current_direction,
                    2.0 * std::f32::consts::PI,
                );
                let relative_new_direction = math::approach_velocity(
                    0.0,
                    relative_approach_direction.signum() * params.streaming_angular_acceleration,
                    delta_time,
                    relative_approach_direction,
                );
                let new_direction = current_direction + relative_new_direction;
                controller.velocity =
                    new_speed * Vec3::new(new_direction.cos(), new_direction.sin(), 0.0);
            }
        }
    }
    if inputs.just_pressed_jump {
        match jump_mode {
            JumpMode::None => {}
            JumpMode::Regular { factor } => {
                controller.velocity.y += factor * params.base_movement.jump_factor;
                commands.entity(entity).remove::<Attachment>();
                commands.entity(entity).insert(SpecialMove::Jumping);
            }
            JumpMode::Dive {
                upward_factor,
                horizontal_factor,
            } => {
                let input_direction = Vec3::new(inputs.input_direction_x, 0.0, 0.0);
                controller.velocity.y += upward_factor * params.base_movement.jump_factor;
                controller.velocity += horizontal_factor * input_direction;
                commands.entity(entity).remove::<Attachment>();
                commands.entity(entity).insert(SpecialMove::Diving);
            }
            JumpMode::Shallow { factor } => {
                controller.velocity.y += factor * params.base_movement.jump_factor;
                commands.entity(entity).remove::<Attachment>();
                commands.entity(entity).insert(SpecialMove::Jumping);
            }
            JumpMode::AwayFromNormal {
                normal,
                upward_impulse_factor,
            } => {
                controller.velocity.y += upward_impulse_factor * params.base_movement.jump_factor;
                controller.velocity +=
                    normal.with_y(0.0).normalize_or_zero() * params.wall_jump_outward_impulse;
                commands.entity(entity).remove::<Attachment>();
                commands.entity(entity).insert(SpecialMove::Jumping);
            }
            JumpMode::Climb { normal } => {
                commands.entity(entity).insert(SpecialMove::Climbing {
                    progress: 0.0,
                    normal,
                });
            }
        }
    }
    if inputs.just_pressed_secondary {
        match secondary_button_mode {
            SecondaryButtonMode::None => {}
            SecondaryButtonMode::EnterRoll => {
                commands
                    .entity(entity)
                    .insert(SpecialMove::Rolling { progress: 0.0 });
            }
            SecondaryButtonMode::KickFromWall { normal } => {
                controller.velocity +=
                    normal.with_y(0.0).normalize_or_zero() * params.wall_jump_outward_impulse;
            }
            SecondaryButtonMode::StartStreaming {
                water_volume_entity,
            } => {
                controller.velocity += params.streaming_impulse
                    * Vec3::new(inputs.input_direction_x, inputs.input_direction_y, 0.0);
                commands
                    .entity(entity)
                    .insert(Attachment::Submerged {
                        water_volume_entity,
                    })
                    .insert(SpecialMove::Diving);
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
    water_and_player_entities: Res<PlayerAndWaterEntities>,
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

    let from_position = match attachment {
        Some(Attachment::Floating { .. }) => {
            let internal_cast_distance = 0.5 * PLAYER_HEIGHT;
            transform.translation + internal_cast_distance * Vec3::Y
        }
        _ => transform.translation,
    };
    let entity_filter = match attachment {
        Some(Attachment::Floating { .. }) => {
            SpatialQueryFilter::default().with_excluded_entities([entity])
        }
        _ => SpatialQueryFilter::default()
            .with_excluded_entities(water_and_player_entities.excluded_entities()),
    };

    let result = move_and_collide_and_slide(
        spatial_queries,
        params,
        &shape,
        &entity_filter,
        from_position,
        attempted_displacement,
        0,
        controller.velocity.length(),
        is_grounded,
    );
    let travel = result.new_position - from_position;
    transform.translation += travel;
    if let Some((attachment, special_move)) = result.new_attachment {
        commands.entity(entity).insert(attachment);
        if let Some(special_move) = special_move {
            commands.entity(entity).insert(special_move);
        } else {
            commands.entity(entity).remove::<SpecialMove>();
        }
        commands.entity(entity).remove::<SpecialMove>();
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
    entity_filter: &SpatialQueryFilter,
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
    let closest_hit = spatial_queries.cast_shape(
        &shape,
        position,
        shape_rotation,
        direction,
        &config,
        &entity_filter,
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

            // Check wall hits and find new attachments if airborne
            if !is_grounded {
                if let Some(movement) = check_aerial_hit_movement(
                    &spatial_queries,
                    &entity_filter,
                    &params,
                    speed,
                    &attempted_displacement,
                    &collision_position,
                    &hit,
                ) {
                    return movement;
                }
            }

            if bounce_no + 1 < params.max_collision_bounces {
                move_and_collide_and_slide(
                    spatial_queries,
                    params,
                    shape,
                    &entity_filter,
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
