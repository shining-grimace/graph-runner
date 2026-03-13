use super::MovementResult;
use crate::controller::{
    Attachment, Facing, PlayerController, SpecialMove, params::CharacterControllerParams,
};
use avian3d::{math::Quaternion, prelude::*};
use bevy::prelude::*;

pub fn check_aerial_hit_movement(
    spatial_queries: &Res<SpatialQueryPipeline>,
    entity_filter: &SpatialQueryFilter,
    params: &Res<CharacterControllerParams>,
    speed: f32,
    attempted_displacement: &Vec3,
    collision_position: &Vec3,
    hit: &ShapeHitData,
) -> Option<MovementResult> {
    if let Some(result) = try_finding_ledge(
        spatial_queries,
        entity_filter,
        params,
        attempted_displacement,
        collision_position,
        hit,
    ) {
        return result;
    }

    if let Some(result) = try_finding_wall(
        params,
        speed,
        attempted_displacement,
        collision_position,
        hit,
    ) {
        return result;
    }

    None
}

/// Cast a shape to look for a grabbable ledge.
/// Returned value None should be considered no-op, while Some(None) should be
/// considerd to mean do not update attachment this frame (no ledge grab and
/// nothing else either).
fn try_finding_ledge(
    spatial_queries: &Res<SpatialQueryPipeline>,
    entity_filter: &SpatialQueryFilter,
    params: &Res<CharacterControllerParams>,
    attempted_displacement: &Vec3,
    collision_position: &Vec3,
    hit: &ShapeHitData,
) -> Option<Option<MovementResult>> {
    let shape = PlayerController::collider(params.collider_skin_thickness);
    let ledge_cast_position = collision_position
        + params.ledge_grab_relative_y * Vec3::Y
        + params.ledge_grab_required_inset * hit.normal2;
    let shape_rotation = Quaternion::default();
    let direction = Dir3::NEG_Y;
    let grabbing_distance = params.ledge_grab_tolerance_y - attempted_displacement.y.min(0.0)
        + params.collider_skin_thickness;
    let config = ShapeCastConfig {
        max_distance: grabbing_distance + params.ledge_grab_relative_y,
        ..default()
    };
    if let Some(ledge_ground) = spatial_queries.cast_shape(
        &shape,
        ledge_cast_position,
        shape_rotation,
        direction,
        &config,
        &entity_filter,
    ) {
        if ledge_ground.distance > grabbing_distance {
            // A ledge is there but we're above it; avoid attaching to this wall
            return Some(None);
        }
        if ledge_ground.distance > 0.0 && ledge_ground.distance > -attempted_displacement.y {
            return Some(Some(MovementResult {
                new_position: collision_position
                    .with_y(ledge_cast_position.y - ledge_ground.distance),
                new_attachment: Some((
                    Attachment::LedgeGrabbed {
                        normal: hit.normal1,
                    },
                    Some(SpecialMove::Halting { progress: 0.0 }),
                )),
                new_velocity: Some(Vec3::ZERO),
            }));
        }
    }

    None
}

fn try_finding_wall(
    params: &Res<CharacterControllerParams>,
    speed: f32,
    attempted_displacement: &Vec3,
    collision_position: &Vec3,
    hit: &ShapeHitData,
) -> Option<Option<MovementResult>> {
    let surface_verticality = hit.normal1.with_y(0.0).normalize_or_zero().dot(hit.normal1);
    if surface_verticality > params.wall_stick_vertical_strictness {
        let incident_angle = attempted_displacement
            .with_y(0.0)
            .normalize_or_zero()
            .dot(hit.normal1);
        if incident_angle < params.wall_stick_angle_threshold {
            let impact_strength = speed
                * attempted_displacement
                    .normalize_or_zero()
                    .dot(hit.normal1)
                    .abs();
            if impact_strength > params.wall_stick_impact_threshold {
                return Some(Some(MovementResult {
                    new_position: *collision_position,
                    new_attachment: Some((
                        Attachment::Walled {
                            normal: hit.normal1,
                            progress: 0.0,
                        },
                        None,
                    )),
                    new_velocity: Some(Vec3::ZERO),
                }));
            }
        }
    }
    None
}

pub fn update_facing(from: &Facing, current_velocity: &Vec3) -> f32 {
    match Vec3::new(current_velocity.x, 0.0, 0.0).try_normalize() {
        Some(travel_facing) => match travel_facing.x > 0.0 {
            true => std::f32::consts::FRAC_PI_2,
            false => -std::f32::consts::FRAC_PI_2,
        },
        None => from.angle,
    }
}
