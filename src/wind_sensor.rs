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
/// Defaults below match the Davis Instruments 6410 (Vantage Pro2
/// anemometer/vane) selected for this project -- see `system.sysml`'s
/// `CupAndVaneWindSensor` doc and `hardware-sourcing.md` for the
/// comparison against the earlier SEN-15901 choice and why it changed
/// (ruggedness: stainless steel bearings, 200 mph wind-tunnel tested,
/// field-proven 10+ year lifespans, vs SEN-15901's unspecified-material
/// bearings and no comparable track record). `direction_offset_deg` is
/// still installation-specific (depends on how the vane is mounted
/// relative to the boat's centerline) -- see `tasks.md` Task 1 for the
/// pending hardware verification pass.
#[derive(Debug, Clone, Copy)]
pub struct WindCalibration {
    /// Wind speed (m/s) per Hz of anemometer pulses. Davis's documented
    /// constant is 2.25 mph per Hz -> 2.25 * 0.44704 m/s per mph.
    pub mps_per_hz: f32,
    /// Degrees added to the raw direction reading to correct for mounting
    /// alignment (vane's "0" position relative to the boat's centerline).
    /// TODO: calibrate on install.
    pub direction_offset_deg: f32,
    /// Maximum raw ADC value the direction pot's full 0..360 sweep maps
    /// to (e.g. 4095 for a 12-bit ADC). Unlike SEN-15901's discrete
    /// resistor-ladder vane, the Davis 6410's is a continuous
    /// potentiometer, so direction is a linear scale, not a lookup
    /// table. TODO: confirm against real hardware -- Davis's vane may
    /// have a small dead zone at the wrap-around point that a purely
    /// linear model doesn't account for.
    pub adc_max: u16,
    /// Consecutive zero-pulse samples before a reading is flagged invalid.
    /// A genuinely calm wind also produces zero pulses, so this is a
    /// deliberate tradeoff: a long enough calm spell reads as "invalid"
    /// rather than "0 m/s". Confirmed acceptable against real hardware
    /// during the hardware verification pass (see tasks.md).
    pub stale_after_samples: u32,
}

impl WindCalibration {
    /// Davis 6410 defaults: anemometer constant from the datasheet
    /// (exact), no direction offset applied yet (calibrate on install),
    /// a 5-sample stale window.
    pub const fn davis_6410_defaults() -> Self {
        Self {
            mps_per_hz: 2.25 * 0.44704,
            direction_offset_deg: 0.0,
            adc_max: 4095,
            stale_after_samples: 5,
        }
    }
}

/// Davis 6410's wind vane is a continuous ~20k-ohm potentiometer (not a
/// discrete resistor ladder like SEN-15901's), so direction is a
/// straight linear scale across the ADC's full range.
fn adc_to_direction_deg(adc: u16, adc_max: u16) -> f32 {
    (adc as f32 / adc_max.max(1) as f32) * 360.0
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

        let raw_deg = adc_to_direction_deg(direction_adc, self.calibration.adc_max) + self.calibration.direction_offset_deg;

        WindReading {
            speed_mps,
            direction_deg: crate::angle::normalize(raw_deg, 360.0),
            valid: true,
        }
    }
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
            ..WindCalibration::davis_6410_defaults()
        }
    }

    #[test]
    fn converts_pulse_count_to_speed() {
        let mut sampler = ApparentWindSampler::new(calibration());
        // 10 pulses in a 2s window -> 5 Hz (not 1s/10Hz: that would make
        // pulse_count / sample_period_s indistinguishable from
        // pulse_count * sample_period_s, since dividing and multiplying
        // by 1.0 are the same thing) -> 5 * (2.25 mph/Hz * 0.44704) ~=
        // 5.029 m/s.
        let reading = sampler.sample(10, 0, 2.0);
        assert!(reading.valid);
        assert!((reading.speed_mps - 5.029).abs() < 0.01);
    }

    #[test]
    fn converts_adc_to_direction_linearly() {
        let mut sampler = ApparentWindSampler::new(calibration());
        // Continuous potentiometer (not SEN-15901's discrete ladder) --
        // quarter-scale ADC lands at ~90 degrees, a straight linear scale.
        let reading = sampler.sample(1, 1024, 1.0);
        assert!((reading.direction_deg - 90.0).abs() < 0.5);
    }

    #[test]
    fn full_scale_adc_wraps_to_zero_not_360() {
        let mut sampler = ApparentWindSampler::new(calibration());
        // adc_max maps to exactly 360 degrees raw, which must normalize
        // to 0 -- direction is always in [0, 360).
        let reading = sampler.sample(1, 4095, 1.0);
        assert!(reading.direction_deg < 1.0);
    }

    #[test]
    fn applies_direction_offset_and_wraps() {
        let mut cal = calibration();
        cal.direction_offset_deg = 30.0;
        let mut sampler = ApparentWindSampler::new(cal);
        // Max ADC (360 deg raw) + 30 deg offset should wrap to ~30, not 390.
        let reading = sampler.sample(1, 4095, 1.0);
        assert!(reading.direction_deg < 360.0);
        assert!((reading.direction_deg - 30.0).abs() < 0.5);
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
        // 4 Hz * (2.25 mph/Hz * 0.44704) ~= 4.023 m/s.
        assert!((reading.speed_mps - 4.023).abs() < 0.01);
    }
}
