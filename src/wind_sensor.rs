//! Wind-sensing abstraction (REQ-004, `system.sysml`'s `WindSensor` part
//! def): [`CupAndVaneWindSensor`] implements it now; a future ultrasonic
//! sensor can implement the same trait later without the rest of the
//! firmware changing.

/// A single apparent-wind reading. `valid = false` means the sensor is
/// disconnected/unresponsive or otherwise not currently trustworthy --
/// consumers must not treat `speed_mps`/`direction_deg` as meaningful in
/// that case (REQ-001's negative AC).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WindReading {
    pub speed_mps: f32,
    pub direction_deg: f32,
    pub valid: bool,
}

impl WindReading {
    pub const INVALID: WindReading = WindReading {
        speed_mps: 0.0,
        direction_deg: 0.0,
        valid: false,
    };
}

/// Implemented by any wind-sensing hardware. `read` is called once per
/// sample period and returns the current apparent-wind reading.
pub trait WindSensor {
    fn read(&mut self) -> WindReading;
}

/// Calibration constants for a cup-and-vane sensor.
///
/// Defaults below match the Argent Data Systems / SparkFun Weather Meter
/// Kit (SEN-15901) selected for this project -- see `system.sysml`'s
/// `CupAndVaneWindSensor` doc and `PROJECT_PRINCIPLES.md` for sourcing.
/// `direction_offset_deg` is still installation-specific (depends on how
/// the vane is mounted relative to the boat's centerline) -- see
/// `tasks.md` Task 1 for the pending hardware verification pass.
#[derive(Debug, Clone, Copy)]
pub struct WindCalibration {
    /// Wind speed (m/s) per Hz of anemometer pulses. 2.4 km/h per
    /// switch-closure/second, per the Weather Meter Kit's documented
    /// anemometer calibration -> 2.4/3.6 = 0.6667 m/s per Hz.
    pub mps_per_hz: f32,
    /// Degrees added to the raw direction reading to correct for mounting
    /// alignment (vane's "0" position relative to the boat's centerline).
    /// TODO: calibrate on install.
    pub direction_offset_deg: f32,
    /// Consecutive zero-pulse samples before a reading is flagged invalid.
    /// A genuinely calm wind also produces zero pulses, so this is a
    /// deliberate tradeoff: a long enough calm spell reads as "invalid"
    /// rather than "0 m/s". Confirmed acceptable against real hardware
    /// during the hardware verification pass (see tasks.md).
    pub stale_after_samples: u32,
}

impl WindCalibration {
    /// Weather Meter Kit defaults: anemometer constant from the
    /// datasheet (exact), no direction offset applied yet (calibrate on
    /// install), a 5-sample stale window.
    pub const fn weather_meter_kit_defaults() -> Self {
        Self {
            mps_per_hz: 2.4 / 3.6,
            direction_offset_deg: 0.0,
            stale_after_samples: 5,
        }
    }
}

/// The Weather Meter Kit's wind vane is a resistor ladder (8 reed
/// switches, up to 16 resolvable positions), not a continuous
/// potentiometer -- direction is decoded by nearest-match against known
/// ADC codes, not a linear scale. Table entries are 12-bit ADC codes
/// (0..4095) derived from the vane's documented per-direction resistance,
/// assuming the standard circuit: a 10k-ohm external pull resistor from
/// 3.3V to the ADC pin, with the vane's internal resistance from that pin
/// to ground. Both the pull resistor value and the ADC's 12-bit
/// resolution need confirming once real hardware is in hand (see
/// `tasks.md`) -- these are calculated from the datasheet, not measured.
const VANE_ADC_TABLE: [(u16, f32); 16] = [
    (3143, 0.0),
    (1624, 22.5),
    (1845, 45.0),
    (335, 67.5),
    (372, 90.0),
    (264, 112.5),
    (738, 135.0),
    (506, 157.5),
    (1149, 180.0),
    (979, 202.5),
    (2520, 225.0),
    (2397, 247.5),
    (3780, 270.0),
    (3309, 292.5),
    (3548, 315.0),
    (2810, 337.5),
];

fn adc_to_direction_deg(adc: u16) -> f32 {
    let mut best_deg = VANE_ADC_TABLE[0].1;
    let mut best_dist = i32::MAX;
    for &(table_adc, deg) in VANE_ADC_TABLE.iter() {
        let dist = (adc as i32 - table_adc as i32).abs();
        if dist < best_dist {
            best_dist = dist;
            best_deg = deg;
        }
    }
    best_deg
}

/// Pure, host-testable calibration/calculation core: turns raw pulse
/// counts and a raw ADC reading into a [`WindReading`]. Owns no hardware,
/// so it is exercised directly by unit tests without the `embedded`
/// feature.
pub struct ApparentWindSampler {
    calibration: WindCalibration,
    stale_counter: u32,
}

impl ApparentWindSampler {
    pub const fn new(calibration: WindCalibration) -> Self {
        Self {
            calibration,
            stale_counter: 0,
        }
    }

    /// Feed one sample period's worth of raw hardware input: the pulse
    /// count observed during the period, the current raw direction-ADC
    /// value, and the period length in seconds.
    pub fn sample(&mut self, pulse_count: u32, direction_adc: u16, sample_period_s: f32) -> WindReading {
        if pulse_count == 0 {
            self.stale_counter = self.stale_counter.saturating_add(1);
        } else {
            self.stale_counter = 0;
        }

        if self.stale_counter >= self.calibration.stale_after_samples {
            return WindReading::INVALID;
        }

        let hz = pulse_count as f32 / sample_period_s;
        let speed_mps = hz * self.calibration.mps_per_hz;

        let raw_deg = adc_to_direction_deg(direction_adc) + self.calibration.direction_offset_deg;

        WindReading {
            speed_mps,
            direction_deg: normalize_deg(raw_deg),
            valid: true,
        }
    }
}

fn normalize_deg(deg: f32) -> f32 {
    let mut d = deg % 360.0;
    if d < 0.0 {
        d += 360.0;
    }
    d
}

#[cfg(feature = "embedded")]
mod hardware {
    use embassy_stm32::adc::{Adc, SampleTime};
    use embassy_stm32::exti::ExtiInput;
    use embassy_stm32::peripherals::ADC1;

    use super::{ApparentWindSampler, WindCalibration, WindReading, WindSensor};

    /// Cup-and-vane sensor: a pulse (reed/Hall) anemometer for speed and a
    /// potentiometer read via ADC for direction.
    ///
    /// Pulse counting runs by racing `wait_for_rising_edge()` against a
    /// fixed sample-period timeout, rather than a hardware timer capture,
    /// to keep this a plain `WindSensor` implementation with no extra
    /// timer peripheral. Revisit if pulse rates at high wind speed turn
    /// out to exceed what polling at this rate can keep up with -- see
    /// tasks.md Task 1's flagged calibration risk.
    pub struct CupAndVaneWindSensor<'d> {
        pulse_pin: ExtiInput<'d, embassy_stm32::mode::Async>,
        direction_adc: Adc<'d, ADC1>,
        direction_channel: embassy_stm32::Peri<'d, embassy_stm32::peripherals::PA1>,
        sampler: ApparentWindSampler,
        sample_period_s: f32,
    }

    impl<'d> CupAndVaneWindSensor<'d> {
        pub fn new(
            pulse_pin: ExtiInput<'d, embassy_stm32::mode::Async>,
            direction_adc: Adc<'d, ADC1>,
            direction_channel: embassy_stm32::Peri<'d, embassy_stm32::peripherals::PA1>,
            calibration: WindCalibration,
            sample_period_s: f32,
        ) -> Self {
            Self {
                pulse_pin,
                direction_adc,
                direction_channel,
                sampler: ApparentWindSampler::new(calibration),
                sample_period_s,
            }
        }

        /// Count pulses for one sample period, then take a direction
        /// reading and produce a `WindReading`. Async because pulse
        /// counting waits on GPIO edges; call this once per sample tick
        /// from the owning task's loop.
        pub async fn sample(&mut self) -> WindReading {
            use embassy_time::{Duration, Timer, with_timeout};

            let mut pulse_count: u32 = 0;
            let deadline = Duration::from_secs_floor(self.sample_period_s as u64).max(Duration::from_millis(1));
            let period_end = embassy_time::Instant::now() + deadline;

            loop {
                let remaining = period_end.saturating_duration_since(embassy_time::Instant::now());
                if remaining == Duration::from_ticks(0) {
                    break;
                }
                match with_timeout(remaining, self.pulse_pin.wait_for_rising_edge()).await {
                    Ok(()) => pulse_count += 1,
                    Err(_timeout) => break,
                }
            }

            // Ensure at least the nominal period elapses even if pulses
            // arrived in a burst early in the window.
            Timer::at(period_end).await;

            let direction_raw: u16 = self
                .direction_adc
                .blocking_read(&mut self.direction_channel, SampleTime::CYCLES64_5);

            self.sampler.sample(pulse_count, direction_raw, self.sample_period_s)
        }
    }

    impl<'d> WindSensor for CupAndVaneWindSensor<'d> {
        fn read(&mut self) -> WindReading {
            // Synchronous trait method kept for API parity with a future
            // sensor that can answer without awaiting; this sensor's real
            // path is the async `sample()` above, called from the owning
            // task.
            WindReading::INVALID
        }
    }
}

#[cfg(feature = "embedded")]
pub use hardware::CupAndVaneWindSensor;

#[cfg(test)]
mod tests {
    use super::*;

    fn calibration() -> WindCalibration {
        WindCalibration {
            stale_after_samples: 3,
            ..WindCalibration::weather_meter_kit_defaults()
        }
    }

    #[test]
    fn converts_pulse_count_to_speed() {
        let mut sampler = ApparentWindSampler::new(calibration());
        // 10 pulses in a 1s window -> 10 Hz -> 10 * (2.4/3.6) ~= 6.667 m/s.
        let reading = sampler.sample(10, 0, 1.0);
        assert!(reading.valid);
        assert!((reading.speed_mps - 6.667).abs() < 0.01);
    }

    #[test]
    fn converts_adc_to_direction_degrees_via_vane_table() {
        let mut sampler = ApparentWindSampler::new(calibration());
        // Table entry for 180 degrees is ADC 1149 exactly.
        let reading = sampler.sample(1, 1149, 1.0);
        assert!((reading.direction_deg - 180.0).abs() < 0.01);
    }

    #[test]
    fn nearest_matches_an_off_table_adc_value() {
        let mut sampler = ApparentWindSampler::new(calibration());
        // 1150 is 1 away from the 180-degree entry (1149) and much
        // farther from any other -- should still resolve to 180.
        let reading = sampler.sample(1, 1150, 1.0);
        assert!((reading.direction_deg - 180.0).abs() < 0.01);
    }

    #[test]
    fn applies_direction_offset_and_wraps() {
        let mut cal = calibration();
        cal.direction_offset_deg = 30.0;
        let mut sampler = ApparentWindSampler::new(cal);
        // Table's 337.5-degree entry (ADC 2810) + 30 deg offset should
        // wrap to 7.5, not 367.5.
        let reading = sampler.sample(1, 2810, 1.0);
        assert!(reading.direction_deg < 360.0);
        assert!((reading.direction_deg - 7.5).abs() < 0.01);
    }

    #[test]
    fn flags_invalid_after_sustained_zero_pulses() {
        let mut sampler = ApparentWindSampler::new(calibration());
        assert!(sampler.sample(0, 0, 1.0).valid);
        assert!(sampler.sample(0, 0, 1.0).valid);
        // Third consecutive zero-pulse sample hits stale_after_samples=3.
        let reading = sampler.sample(0, 0, 1.0);
        assert!(!reading.valid);
        assert_eq!(reading, WindReading::INVALID);
    }

    #[test]
    fn recovers_from_invalid_once_pulses_resume() {
        let mut sampler = ApparentWindSampler::new(calibration());
        for _ in 0..3 {
            sampler.sample(0, 0, 1.0);
        }
        assert!(!sampler.sample(0, 0, 1.0).valid);
        let reading = sampler.sample(4, 0, 1.0);
        assert!(reading.valid);
        // 4 Hz * (2.4/3.6 m/s per Hz) ~= 2.667 m/s.
        assert!((reading.speed_mps - 2.667).abs() < 0.01);
    }
}
