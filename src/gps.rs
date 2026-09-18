//! Onboard GPS handling (REQ-003): parses the GPS module's NMEA0183 stream
//! into position/speed-over-ground/course-over-ground fixes, independent
//! of the boat's own instrument bus.

use nmea0183::{ParseResult, Parser, RMC};

/// A single GPS fix. `valid = false` means no fix is currently available
/// (never acquired, or the last one aged past `max_fix_age_s`) --
/// consumers must not treat the other fields as meaningful in that case.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GpsFix {
    pub latitude_deg: f64,
    pub longitude_deg: f64,
    pub sog_mps: f32,
    pub cog_deg: f32,
    pub valid: bool,
}

impl GpsFix {
    pub const INVALID: GpsFix = GpsFix {
        latitude_deg: 0.0,
        longitude_deg: 0.0,
        sog_mps: 0.0,
        cog_deg: 0.0,
        valid: false,
    };
}

/// Extracts a [`GpsFix`] from a parsed RMC sentence. Returns `None` when
/// the receiver has no course over ground (some receivers omit it while
/// stationary) -- true wind (REQ-002) needs a full velocity vector, so a
/// position-only fix is not exposed as usable here.
fn fix_from_rmc(rmc: &RMC) -> Option<GpsFix> {
    let course = rmc.course.as_ref()?;
    Some(GpsFix {
        latitude_deg: rmc.latitude.as_f64(),
        longitude_deg: rmc.longitude.as_f64(),
        sog_mps: rmc.speed.as_mps(),
        cog_deg: course.degrees,
        valid: true,
    })
}

/// Pure, host-testable GPS fix tracker: feed it raw NMEA0183 bytes as they
/// arrive, and ask it for the current fix once per sample tick. Owns no
/// hardware, so it is exercised directly by unit tests against captured
/// sentences without the `embedded` feature.
pub struct GpsFixTracker {
    parser: Parser,
    last_fix: Option<GpsFix>,
    seconds_since_fix: f32,
    max_fix_age_s: f32,
}

impl GpsFixTracker {
    pub fn new(max_fix_age_s: f32) -> Self {
        Self {
            parser: Parser::new(),
            last_fix: None,
            seconds_since_fix: f32::MAX,
            max_fix_age_s,
        }
    }

    /// Feed raw bytes as they arrive from the GPS UART. Updates the
    /// internal last-known fix whenever a complete, usable RMC sentence
    /// is parsed; other sentence types and parse errors are ignored (the
    /// parser already validates checksums).
    pub fn feed_bytes(&mut self, bytes: &[u8]) {
        for result in self.parser.parse_from_bytes(bytes) {
            if let Ok(ParseResult::RMC(Some(rmc))) = result {
                if let Some(fix) = fix_from_rmc(&rmc) {
                    self.last_fix = Some(fix);
                    self.seconds_since_fix = 0.0;
                }
            }
        }
    }

    /// Called once per sample tick with the elapsed seconds since the
    /// previous call, to age out a fix that's gone stale (REQ-003's
    /// negative case: no/stale fix -> invalid, not a frozen last position).
    pub fn sample(&mut self, elapsed_s: f32) -> GpsFix {
        self.seconds_since_fix += elapsed_s;
        match self.last_fix {
            Some(fix) if self.seconds_since_fix <= self.max_fix_age_s => fix,
            _ => GpsFix::INVALID,
        }
    }
}

#[cfg(feature = "embedded")]
mod hardware {
    use embassy_stm32::usart::UartRx;
    use embassy_time::Instant;

    use super::{GpsFix, GpsFixTracker};

    /// Owns the GPS module's UART RX half; reads bytes as they arrive and
    /// hands back the current fix once per sample tick.
    pub struct GpsReceiver<'d> {
        rx: UartRx<'d, embassy_stm32::mode::Async>,
        tracker: GpsFixTracker,
        last_sample_at: Instant,
    }

    impl<'d> GpsReceiver<'d> {
        pub fn new(rx: UartRx<'d, embassy_stm32::mode::Async>, max_fix_age_s: f32) -> Self {
            Self {
                rx,
                tracker: GpsFixTracker::new(max_fix_age_s),
                last_sample_at: Instant::now(),
            }
        }

        /// Drains whatever bytes are currently available (non-blocking,
        /// bounded by `read_until_idle`'s own idle timeout) then returns
        /// the current fix. Call this once per sample tick from the
        /// owning task's loop.
        pub async fn sample(&mut self) -> GpsFix {
            let mut buf = [0u8; 128];
            if let Ok(n) = self.rx.read_until_idle(&mut buf).await {
                self.tracker.feed_bytes(&buf[..n]);
            }

            let now = Instant::now();
            let elapsed_s = now.saturating_duration_since(self.last_sample_at).as_millis() as f32 / 1000.0;
            self.last_sample_at = now;

            self.tracker.sample(elapsed_s)
        }
    }
}

#[cfg(feature = "embedded")]
pub use hardware::GpsReceiver;

#[cfg(test)]
mod tests {
    use super::*;

    // A real RMC sentence with a valid fix and course (boat underway).
    const RMC_WITH_FIX: &[u8] = b"$GPRMC,125504.049,A,5542.2389,N,03741.6063,E,5.50,90.00,200906,,,A*54\r\n";
    // GGA-only sentence: no RMC, so no fix should be extracted from it.
    const GGA_ONLY: &[u8] = b"$GPGGA,145659.00,5956.695396,N,03022.454999,E,2,07,0.6,9.0,M,18.0,M,,*62\r\n";

    #[test]
    fn no_fix_before_any_sentence_received() {
        let mut tracker = GpsFixTracker::new(5.0);
        let fix = tracker.sample(0.0);
        assert!(!fix.valid);
    }

    #[test]
    fn extracts_fix_from_rmc_sentence() {
        let mut tracker = GpsFixTracker::new(5.0);
        tracker.feed_bytes(RMC_WITH_FIX);
        let fix = tracker.sample(0.0);
        assert!(fix.valid);
        assert!((fix.sog_mps - 2.83).abs() < 0.05); // 5.50 kn ~= 2.83 m/s
        assert!((fix.cog_deg - 90.0).abs() < 0.01);
    }

    #[test]
    fn non_rmc_sentences_do_not_produce_a_fix() {
        let mut tracker = GpsFixTracker::new(5.0);
        tracker.feed_bytes(GGA_ONLY);
        let fix = tracker.sample(0.0);
        assert!(!fix.valid);
    }

    #[test]
    fn fix_ages_out_after_max_age() {
        let mut tracker = GpsFixTracker::new(2.0);
        tracker.feed_bytes(RMC_WITH_FIX);
        assert!(tracker.sample(1.0).valid);
        // Cumulative elapsed time now exceeds max_fix_age_s with no new fix.
        let fix = tracker.sample(1.5);
        assert!(!fix.valid);
        assert_eq!(fix, GpsFix::INVALID);
    }

    #[test]
    fn fix_refreshes_on_new_sentence_before_it_goes_stale() {
        let mut tracker = GpsFixTracker::new(2.0);
        tracker.feed_bytes(RMC_WITH_FIX);
        tracker.sample(1.5);
        tracker.feed_bytes(RMC_WITH_FIX);
        let fix = tracker.sample(1.5);
        assert!(fix.valid);
    }
}
