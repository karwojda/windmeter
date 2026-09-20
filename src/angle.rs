//! Shared angle-wrapping helper. `wind_sensor.rs`, `wind_compute.rs`, and
//! `nmea2000.rs` each independently reimplemented "wrap a signed value
//! into `[0, modulus)`" (once for degrees, once for radians) -- mutation
//! testing found the same missed-boundary gap in all three copies, which
//! is exactly the kind of duplication that should have been one function
//! tested once, not three nearly-identical ones tested three times.

/// Wraps `value` into `[0, modulus)` via floating-point remainder,
/// folding a negative remainder back into range.
pub(crate) fn normalize(value: f32, modulus: f32) -> f32 {
    let mut v = value % modulus;
    if v < 0.0 {
        v += modulus;
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn value_already_in_range_is_unchanged() {
        assert!((normalize(75.0, 360.0) - 75.0).abs() < 1e-6);
    }

    #[test]
    fn negative_value_wraps_up_into_range() {
        // Distinguishes the wrap-add from every other arithmetic mutant
        // on it (+= vs -=/*=) and from "only wrap on exactly zero" (==).
        assert!((normalize(-30.0, 360.0) - 330.0).abs() < 1e-6);
    }

    #[test]
    fn value_exactly_at_modulus_wraps_to_zero() {
        // Distinguishes `< 0.0` from `<= 0.0`: the remainder here is
        // exactly 0.0, which must NOT take the wrap-add branch (that
        // would incorrectly produce `modulus` instead of `0`).
        assert!((normalize(360.0, 360.0) - 0.0).abs() < 1e-6);
    }
}
