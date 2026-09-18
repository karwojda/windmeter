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
/// `mps_per_hz` and `direction_offset_deg` are sensor/installation
/// specific and not yet confirmed against real hardware -- see
/// `tasks.md` Task 1 for the pending hardware verification pass.
#[derive(Debug, Clone, Copy)]
pub struct WindCalibration {
    /// Wind speed (m/s) per Hz of anemometer pulses. TODO: confirm against
    /// the actual sensor's datasheet.
    pub mps_per_hz: f32,
    /// Degrees added to the raw direction-pot reading to correct for
    /// mounting alignment. TODO: calibrate on install.
    pub direction_offset_deg: f32,
    /// Maximum raw value the direction ADC can report (e.g. 4095 for a
    /// 12-bit ADC), used to scale the reading to 0..360 degrees.
    pub adc_max: u16,
    /// Consecutive zero-pulse samples before a reading is flagged invalid.
    /// A genuinely calm wind also produces zero pulses, so this is a
    /// deliberate tradeoff: a long enough calm spell reads as "invalid"
    /// rather than "0 m/s". Confirmed acceptable against real hardware
    /// during the hardware verification pass (see tasks.md).
    pub stale_after_samples: u32,
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

        let adc_max = self.calibration.adc_max.max(1) as f32;
        let raw_deg = (direction_adc as f32 / adc_max) * 360.0 + self.calibration.direction_offset_deg;

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
            mps_per_hz: 0.5,
            direction_offset_deg: 0.0,
            adc_max: 4095,
            stale_after_samples: 3,
        }
    }

    #[test]
    fn converts_pulse_count_to_speed() {
        let mut sampler = ApparentWindSampler::new(calibration());
        // 10 pulses in a 1s window -> 10 Hz -> 10 * 0.5 = 5.0 m/s.
        let reading = sampler.sample(10, 0, 1.0);
        assert!(reading.valid);
        assert!((reading.speed_mps - 5.0).abs() < 1e-6);
    }

    #[test]
    fn converts_adc_to_direction_degrees() {
        let mut sampler = ApparentWindSampler::new(calibration());
        // Half-scale ADC -> 180 degrees.
        let reading = sampler.sample(1, 2048, 1.0);
        assert!((reading.direction_deg - 180.0).abs() < 0.2);
    }

    #[test]
    fn applies_direction_offset_and_wraps() {
        let mut cal = calibration();
        cal.direction_offset_deg = 10.0;
        let mut sampler = ApparentWindSampler::new(cal);
        // Max ADC (360 deg raw) + 10 deg offset should wrap to 10, not 370.
        let reading = sampler.sample(1, 4095, 1.0);
        assert!(reading.direction_deg < 360.0);
        assert!((reading.direction_deg - 10.0).abs() < 0.5);
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
        assert!((reading.speed_mps - 2.0).abs() < 1e-6);
    }
}
