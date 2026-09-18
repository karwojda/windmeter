//! True wind computation (REQ-002, `domain.sysml`'s `Boat.trueWind`):
//! combines a live apparent-wind reading with a live GPS fix into true
//! wind, via the classic wind-triangle vector subtraction.

use crate::gps::GpsFix;
use crate::wind_sensor::WindReading;

/// `trueWind = apparentWind - boatVelocity` (`domain.sysml`).
///
/// Documented v1 limitation (see `requirements.md`, REQ-002): with no
/// heading sensor, this treats GPS course-over-ground as a stand-in for
/// the boat's heading (i.e. assumes no current, no leeway) to put
/// apparent wind's boat-relative direction into the same north-relative
/// frame as the GPS velocity vector before subtracting.
///
/// Either input invalid -> `WindReading::INVALID`, never computed from a
/// bad reading.
pub fn compute_true_wind(apparent: WindReading, gps: GpsFix) -> WindReading {
    if !apparent.valid || !gps.valid {
        return WindReading::INVALID;
    }

    let apparent_true_deg = normalize_deg(apparent.direction_deg + gps.cog_deg);
    let (aw_east, aw_north) = to_cartesian(apparent.speed_mps, apparent_true_deg);
    let (bv_east, bv_north) = to_cartesian(gps.sog_mps, gps.cog_deg);

    let tw_east = aw_east - bv_east;
    let tw_north = aw_north - bv_north;

    WindReading {
        speed_mps: sqrt(tw_east * tw_east + tw_north * tw_north),
        direction_deg: normalize_deg(atan2_deg(tw_east, tw_north)),
        valid: true,
    }
}

/// Compass bearing (0 = North, clockwise) + speed -> (east, north) components.
fn to_cartesian(speed: f32, bearing_deg: f32) -> (f32, f32) {
    let rad = to_radians(bearing_deg);
    (speed * sin(rad), speed * cos(rad))
}

/// atan2(east, north) as a compass bearing in degrees, unnormalized.
fn atan2_deg(east: f32, north: f32) -> f32 {
    to_degrees(atan2(east, north))
}

fn normalize_deg(deg: f32) -> f32 {
    let mut d = deg % 360.0;
    if d < 0.0 {
        d += 360.0;
    }
    d
}

fn to_radians(deg: f32) -> f32 {
    deg * (core::f32::consts::PI / 180.0)
}

fn to_degrees(rad: f32) -> f32 {
    rad * (180.0 / core::f32::consts::PI)
}

// core has no floating-point transcendental functions (they normally come
// from std's link to the platform libm); `libm` provides the same
// functions as free functions so this stays no_std-compatible on the
// embedded target while remaining plain f32 math on host.
fn sin(x: f32) -> f32 {
    libm::sinf(x)
}
fn cos(x: f32) -> f32 {
    libm::cosf(x)
}
fn atan2(y: f32, x: f32) -> f32 {
    libm::atan2f(y, x)
}
fn sqrt(x: f32) -> f32 {
    libm::sqrtf(x)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wind(speed: f32, direction: f32) -> WindReading {
        WindReading {
            speed_mps: speed,
            direction_deg: direction,
            valid: true,
        }
    }

    fn fix(sog: f32, cog: f32) -> GpsFix {
        GpsFix {
            latitude_deg: 0.0,
            longitude_deg: 0.0,
            sog_mps: sog,
            cog_deg: cog,
            valid: true,
        }
    }

    #[test]
    fn boat_stationary_true_wind_equals_apparent_wind() {
        // No boat velocity -> apparent wind rotated into true frame IS
        // true wind (COG is meaningless at 0 speed, but 0 SOG means the
        // subtracted vector is zero regardless of COG).
        let apparent = wind(5.0, 45.0);
        let true_wind = compute_true_wind(apparent, fix(0.0, 0.0));
        assert!(true_wind.valid);
        assert!((true_wind.speed_mps - 5.0).abs() < 0.01);
        assert!((true_wind.direction_deg - 45.0).abs() < 0.5);
    }

    #[test]
    fn boat_sailing_dead_downwind_true_wind_is_slower() {
        // Boat doing 3 m/s due north (COG 0), apparent wind dead ahead
        // (boat-relative 0 deg, so true-frame direction = 0 + 0 = north)
        // at 8 m/s -- classic case: true wind = apparent - boat speed,
        // same direction, when running directly downwind.
        let apparent = wind(8.0, 0.0);
        let true_wind = compute_true_wind(apparent, fix(3.0, 0.0));
        assert!(true_wind.valid);
        assert!((true_wind.speed_mps - 5.0).abs() < 0.05);
        assert!(true_wind.direction_deg < 1.0 || true_wind.direction_deg > 359.0);
    }

    #[test]
    fn invalid_apparent_wind_yields_invalid_true_wind() {
        let true_wind = compute_true_wind(WindReading::INVALID, fix(3.0, 90.0));
        assert!(!true_wind.valid);
    }

    #[test]
    fn invalid_gps_fix_yields_invalid_true_wind() {
        let true_wind = compute_true_wind(wind(5.0, 45.0), GpsFix::INVALID);
        assert!(!true_wind.valid);
    }
}
