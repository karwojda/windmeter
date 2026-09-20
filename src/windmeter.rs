use crate::moving_average::MovingAverage;
use crate::wind_sensor::WindReading;

/// Smoothed apparent-wind state, fed one raw [`WindReading`] at a time
/// (typically from a [`crate::wind_sensor::WindSensor`]).
pub struct Wind<const N: usize> {
    direction: f32,
    speed: f32,
    valid: bool,

    filtered_speed: f32,
    filtered_direction: f32,

    speed_buffer: MovingAverage<N>,
}

impl<const N: usize> Wind<N> {
    pub fn new() -> Self {
        Wind {
            direction: 0.0,
            speed: 0.0,
            valid: false,
            filtered_speed: 0.0,
            filtered_direction: 0.0,
            speed_buffer: MovingAverage::new(),
        }
    }

    /// Feed one sensor reading into the filter. An invalid reading leaves
    /// the smoothed speed/direction untouched (so a momentary dropout
    /// doesn't yank the average toward zero) but does mark the wind state
    /// invalid so consumers stop trusting it (REQ-001's negative AC).
    pub fn update(&mut self, reading: WindReading) {
        self.valid = reading.valid;
        if reading.valid {
            self.speed = reading.speed_mps;
            self.direction = reading.direction_deg;
            self.filter_speed();
            self.filter_direction();
        }
    }

    fn filter_speed(&mut self) {
        self.filtered_speed = self.speed_buffer.update(self.speed);
    }

    // Direction is circular (0/360 wrap) -- a naive moving average of the
    // raw degrees would be wrong near the wrap (e.g. averaging 359 and 1
    // should read ~0, not 180). Rather than pull in a trig dependency for
    // a proper circular mean, v1 passes the latest valid reading through
    // unsmoothed.
    fn filter_direction(&mut self) {
        self.filtered_direction = self.direction;
    }

    pub fn filtered_speed(&self) -> f32 {
        self.filtered_speed
    }

    pub fn filtered_direction(&self) -> f32 {
        self.filtered_direction
    }

    pub fn is_valid(&self) -> bool {
        self.valid
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid(speed: f32, direction: f32) -> WindReading {
        WindReading {
            speed_mps: speed,
            direction_deg: direction,
            valid: true,
        }
    }

    #[test]
    fn starts_invalid_with_no_readings() {
        let wind: Wind<4> = Wind::new();
        assert!(!wind.is_valid());
    }

    #[test]
    fn smooths_speed_over_the_window() {
        let mut wind: Wind<2> = Wind::new();
        wind.update(valid(2.0, 90.0));
        wind.update(valid(4.0, 90.0));
        assert!(wind.is_valid());
        assert!((wind.filtered_speed() - 3.0).abs() < 1e-6);
    }

    #[test]
    fn invalid_reading_marks_state_invalid_without_moving_average() {
        let mut wind: Wind<4> = Wind::new();
        wind.update(valid(5.0, 45.0));
        wind.update(WindReading::INVALID);
        assert!(!wind.is_valid());
        // Last good smoothed speed is preserved, not zeroed, so a
        // momentary dropout doesn't look like a sudden calm.
        assert!((wind.filtered_speed() - 5.0).abs() < 1e-6);
    }

    #[test]
    fn direction_passes_through_latest_valid_reading() {
        // 45.0, not 1.0: a value distinct from any constant a mutant
        // might substitute (an earlier version of this test happened to
        // expect exactly 1.0, which let a "replace filtered_direction()
        // with 1.0" mutant pass undetected).
        let mut wind: Wind<4> = Wind::new();
        wind.update(valid(1.0, 359.0));
        wind.update(valid(1.0, 45.0));
        assert!((wind.filtered_direction() - 45.0).abs() < 1e-6);
    }
}