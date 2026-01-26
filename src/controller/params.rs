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

    /// Movement parameters while landing from a big fall
    pub landing_movement: Manoeuvrability,

    /// Movement parameters while coming to a halt (e.g. changing direction sharply)
    pub halting_movement: Manoeuvrability,

    /// Movement parameters while sliding on ground
    pub sliding_movement: Manoeuvrability,

    /// Movement parameters while rolling on ground
    pub rolling_movement: Manoeuvrability,

    /// Movement parameters while underwater
    pub submerged_movement: Manoeuvrability,

    /// Movement parameters while floating on the surface of water
    pub floating_movement: Manoeuvrability,

    /// The speed above which running occurs
    pub running_speed: Scalar,

    /// The proportion of gravity that applies while jumping
    pub jumping_gravity_factor: Scalar,

    /// The proportion of gravity that applies while rolling in the air
    pub aerial_roll_gravity_factor: Scalar,

    /// The rate at which move progress advances (relative to normal) while rolling in the air
    pub aerial_roll_progress_rate: Scalar,

    /// Impulse factor (relative to base jump) of a wall jump in the Y direction if sliding down the wall
    pub shallow_wall_jump_upward_factor: Scalar,

    /// Impulse of a wall jump away from the wall's normal
    pub wall_jump_outward_impulse: Scalar,

    /// Initial impulse applied when starting to stream underwater
    pub streaming_impulse: Scalar,

    /// Angular acceleration changing direction while streaming through water
    pub streaming_angular_acceleration: Scalar,

    /// Gravity while falling
    pub gravity: Vector,

    /// Amount to shrink the collider during shape casts to prevent various issues
    pub collider_skin_thickness: Scalar,

    /// The number of surfaces that collide-and-slide can slide along in a single step
    pub max_collision_bounces: usize,

    /// The incident dot product above which the player can escape (no ground attachment can occur)
    pub escape_incidence: Scalar,

    /// Upward "gravity" while underwater
    pub buoyant_gravity: Vector,

    /// Terminal velocity while falling
    pub terminal_velocity: Scalar,

    /// Terminal velocity while floating upwards
    pub buoyant_terminal_velocity: Scalar,

    /// Terminal velocity while sliding down a wall
    pub wall_slide_terminal_velocity: Scalar,

    /// The proportion of the regular upward jump impulse applicable when diving
    pub dive_jump_upward_factor: Scalar,

    /// The proportion of the regular upward impulse applicable horizontally when diving
    pub dive_jump_horizontal_factor: Scalar,

    /// Maximum slope angle that permits walking
    pub max_walking_slope_angle: Scalar,

    /// Maximum slope angle that is considered ground at all
    pub max_sliding_slope_angle: Scalar,

    /// The closeness to a perfectly-vertical wall needed for sticking to
    pub wall_stick_vertical_strictness: Scalar,

    /// The incident angle dot product threshold below which a wall sticks
    pub wall_stick_angle_threshold: Scalar,

    /// The incident speed threshold above which a wall sticks
    pub wall_stick_impact_threshold: Scalar,

    /// Time spent rolling before it unrolls
    pub rolling_move_duration: f32,

    /// Time prevented from movement after landing hard
    pub landing_move_duration: f32,

    /// Time prevented from moving while halting
    pub halting_move_duration: f32,

    /// Time prevented from movement while climbing a ledge
    pub climbing_move_duration: f32,

    /// Time prevented from movement after becoming submerged
    pub submersion_move_duration: f32,

    /// Time to stick to a wall before sliding down
    pub wall_stick_duration: f32,

    /// The vertical speed above which landing will stall the player
    pub landing_stall_speed_threshold: Scalar,

    // The horizontal speed above which landing will go into a roll
    pub landing_roll_speed_threshold: Scalar,

    /// The horizontal speed below which a roll will end
    pub unroll_speed_threshold: Scalar,

    /// The horizontal speed below which a slide on ground will end
    pub unslide_speed_threshold: Scalar,
}

impl Default for CharacterControllerParams {
    fn default() -> Self {
        Self {
            base_movement: Manoeuvrability {
                input_factor: 5.0,
                reverse_input_factor: 10.0,
                jump_factor: 10.0,
                max_speed_factor: 5.0,
                stop_factor: 0.5,
            },
            ground_movement: Manoeuvrability {
                input_factor: 1.0,
                reverse_input_factor: 3.0,
                jump_factor: 1.0,
                max_speed_factor: 1.0,
                stop_factor: 1.0,
            },
            aerial_movement: Manoeuvrability {
                input_factor: 0.25,
                reverse_input_factor: 1.0,
                jump_factor: 0.0,
                max_speed_factor: 1.0,
                stop_factor: 10.0,
            },
            landing_movement: Manoeuvrability {
                input_factor: 0.0,
                reverse_input_factor: 1.0,
                jump_factor: 0.0,
                max_speed_factor: 0.0,
                stop_factor: 0.01, // Should not be zero
            },
            halting_movement: Manoeuvrability {
                input_factor: 0.0,
                reverse_input_factor: 15.0,
                jump_factor: 1.5,
                max_speed_factor: 0.0,
                stop_factor: 0.01, // Should not be zero
            },
            sliding_movement: Manoeuvrability {
                input_factor: 0.2,
                reverse_input_factor: 1.0,
                jump_factor: 0.5,
                max_speed_factor: 1.2,
                stop_factor: 3.0,
            },
            rolling_movement: Manoeuvrability {
                input_factor: 0.2,
                reverse_input_factor: 1.0,
                jump_factor: 0.8,
                max_speed_factor: 1.3,
                stop_factor: 3.0,
            },
            submerged_movement: Manoeuvrability {
                input_factor: 0.7,
                reverse_input_factor: 1.0,
                jump_factor: 0.0,
                max_speed_factor: 0.8,
                stop_factor: 0.4,
            },
            floating_movement: Manoeuvrability {
                input_factor: 0.4,
                reverse_input_factor: 1.0,
                jump_factor: 0.75,
                max_speed_factor: 0.6,
                stop_factor: 2.0,
            },
            running_speed: 3.0,
            jumping_gravity_factor: 0.7,
            aerial_roll_gravity_factor: 0.2,
            aerial_roll_progress_rate: 0.5,
            shallow_wall_jump_upward_factor: 0.75,
            wall_jump_outward_impulse: 8.0,
            streaming_impulse: 8.0,
            streaming_angular_acceleration: 5.0,
            gravity: Vector::NEG_Y * 9.81 * 1.8,
            collider_skin_thickness: 0.01,
            max_collision_bounces: 3,
            escape_incidence: std::f32::consts::FRAC_PI_4,
            buoyant_gravity: Vector::Y * 9.81 * 0.4,
            terminal_velocity: -14.0,
            buoyant_terminal_velocity: 4.0,
            wall_slide_terminal_velocity: -2.0,
            dive_jump_upward_factor: 1.5,
            dive_jump_horizontal_factor: 1.5,
            max_walking_slope_angle: std::f32::consts::PI * 0.17,
            max_sliding_slope_angle: std::f32::consts::PI * 0.33,
            wall_stick_vertical_strictness: 0.9,
            wall_stick_angle_threshold: -0.7,
            wall_stick_impact_threshold: 0.5,
            rolling_move_duration: 1.0,
            landing_move_duration: 2.0,
            halting_move_duration: 1.5,
            climbing_move_duration: 3.0,
            submersion_move_duration: 1.0,
            wall_stick_duration: 1.5,
            landing_stall_speed_threshold: 3.0,
            landing_roll_speed_threshold: 3.0,
            unroll_speed_threshold: 1.0,
            unslide_speed_threshold: 1.0,
        }
    }
}
