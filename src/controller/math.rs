use crate::markers::WaterVolumeExtents;
use bevy::prelude::*;

/// Process timestep of acceleration with terminal velocity.
///
/// No damping is applied until the terminal velocity is exceeded, at which point the
/// damping kicks in to gradually correct it.
///
/// Damping is based on coefficient calculated at the terminal velocity:
/// a + -k vterm = 0, hence k = a / vterm
///
/// This is applied like a typical damping force, assuming unit mass:
/// F = dv/dt = -k v
/// dv = -k v dt
pub fn approach_velocity(current: f32, acceleration: f32, delta_time: f32, terminal: f32) -> f32 {
    let new = current + acceleration * delta_time;
    if (terminal - new).signum() == terminal.signum() {
        return new;
    }
    if (terminal - current).signum() != terminal.signum() {
        return terminal;
    }
    let damping = acceleration / terminal;
    let damped_new = new - damping * new * delta_time;
    match terminal.signum() > 0.0 {
        true => damped_new.max(terminal),
        false => damped_new.min(terminal),
    }
}

pub fn approach_zero(current: f32, delta_time: f32, max_speed: f32, stop_time: f32) -> f32 {
    let deceleration = current.signum() * max_speed / stop_time;
    let new = current - deceleration * delta_time;
    match new.signum() == current.signum() {
        true => new,
        false => 0.0,
    }
}

pub fn float_modulus(mut value: f32, range: f32) -> f32 {
    let abs_range = range.abs();
    while value > 0.5 * abs_range {
        value -= abs_range;
    }
    while value < -0.5 * abs_range {
        value += abs_range;
    }
    value
}

pub fn inside_volume(
    point: &Vec3,
    volume_position: &Vec3,
    volume: &WaterVolumeExtents,
    skin_thickness: f32,
) -> bool {
    let relative_x = (point.x - volume_position.x).abs();
    let relative_y = (point.y - volume_position.y).abs();
    let relative_z = (point.z - volume_position.z).abs();
    relative_x < (volume.half_extent_x + skin_thickness)
        && relative_y < (volume.half_extent_y + skin_thickness)
        && relative_z < (volume.half_extent_z + skin_thickness)
}
