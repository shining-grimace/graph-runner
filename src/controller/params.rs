use super::Manoeuvrability;
use avian3d::math::{Scalar, Vector};
use bevy::prelude::*;

#[derive(Resource, Reflect)]
#[reflect(Resource)]
pub struct CharacterControllerParams {
    /// The base numbers for input acceleration, jump impulse
    pub base_movement: Manoeuvrability,

    /// Movement parameters on ground
    pub ground_movement: Manoeuvrability,

    /// Movement parameters in the air
    pub aerial_movement: Manoeuvrability,

    /// Gravity while falling
    pub gravity: Vector,

    /// Amount to shrink the collider during shape casts to prevent various issues
    pub collider_skin_thickness: Scalar,

    /// The number of surfaces that collide-and-slide can slide along in a single step
    pub max_collision_bounces: usize,

    /// The incident dot product above which the player can escape (no ground attachment can occur)
    pub escape_incidence: Scalar,

    /// Terminal velocity while falling
    pub terminal_velocity: Scalar,

    /// Terminal velocity while floating upwards
    pub buoyant_terminal_velocity: Scalar,

    /// Maximum slope angle that permits walking
    pub max_walking_slope_angle: Scalar,

    /// Maximum slope angle that is considered ground at all
    pub max_sliding_slope_angle: Scalar,
}

impl Default for CharacterControllerParams {
    fn default() -> Self {
        Self {
            base_movement: Manoeuvrability {
                input_factor: 5.0,
                reverse_input_factor: 10.0,
                jump_factor: 12.0,
                speed_factor: 3.0,
                stop_factor: 0.5,
            },
            ground_movement: Manoeuvrability {
                input_factor: 1.0,
                reverse_input_factor: 3.0,
                jump_factor: 1.0,
                speed_factor: 1.0,
                stop_factor: 1.0,
            },
            aerial_movement: Manoeuvrability {
                input_factor: 0.25,
                reverse_input_factor: 1.0,
                jump_factor: 0.0,
                speed_factor: 1.0,
                stop_factor: 10.0,
            },
            gravity: Vector::NEG_Y * 9.81 * 2.0,
            collider_skin_thickness: 0.01,
            max_collision_bounces: 3,
            escape_incidence: std::f32::consts::FRAC_PI_4,
            terminal_velocity: -20.0,
            buoyant_terminal_velocity: 4.0,
            max_walking_slope_angle: std::f32::consts::PI * 0.17,
            max_sliding_slope_angle: std::f32::consts::PI * 0.33,
        }
    }
}
