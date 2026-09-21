//! Named simulation scenarios. Each produces a sequence of target
//! *physical* states (apparent wind + the boat's own GPS velocity), in SI
//! units -- `main.rs` converts each tick into the sensor-shaped input
//! (pulse counts, ADC codes, an NMEA sentence) the real pipeline actually
//! consumes, so a scenario only ever states *what should be true*, never
//! how to fake a peripheral.
//!
//! Add a scenario by adding a function + a `Scenario` variant + the two
//! match arms below.

const TICKS: usize = 16;
/// 1 knot in m/s, used to convert a scenario's SI boat speed into knots
/// for the generated NMEA sentence.
const MPS_PER_KNOT: f32 = 0.514444;

/// One simulated tick's target physical state.
#[derive(Debug, Clone, Copy)]
pub struct ScenarioTick {
    /// Apparent wind speed, m/s.
    pub apparent_speed_mps: f32,
    /// Apparent wind direction, degrees, relative to the boat's
    /// centerline (0 = dead ahead) -- matches what the real sensor
    /// reports.
    pub apparent_direction_deg: f32,
    /// Boat speed over ground, m/s (becomes the simulated GPS's SOG).
    pub boat_sog_mps: f32,
    /// Boat course over ground, degrees true (becomes the simulated
    /// GPS's COG).
    pub boat_cog_deg: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scenario {
    /// Apparent wind speed drifts ~3..9 m/s, direction sweeps a full
    /// rotation, against a constant boat velocity (3 kn @ 45 deg). The
    /// original/default scenario -- a general "does the pipeline move
    /// plausibly" smoke test.
    VaryingWind,
    /// True wind is dead calm (0 m/s): with no wind at all, the only
    /// apparent wind a moving boat feels is its own motion, which always
    /// appears dead ahead (boat-relative 0 deg) at exactly the boat's
    /// own speed. Boat speed ramps up over the run; heading stays
    /// constant ("increases its speed in the same direction").
    ///
    /// Computed true wind speed comes out to ~0.0 +/- up to ~0.5 m/s, NOT
    /// a clean 0.0 -- run it and look, don't assume. That residual is
    /// real and expected, not a bug: `WindCalibration::davis_6410_defaults`'s
    /// `mps_per_hz` (~1.006) means apparent speed can only take values
    /// that are integer multiples of it (a whole pulse count per 1s
    /// sample), so this scenario's continuously-ramping target speed
    /// gets rounded to the nearest reachable value before
    /// `compute_true_wind` ever sees it, while the simulated GPS's SOG
    /// (encoded as a two-decimal knots value) doesn't round nearly as
    /// coarsely -- the gap between the two is exactly the anemometer's
    /// own pulse-counting resolution, reproduced faithfully rather than
    /// smoothed away. A real device at this sample rate would show the
    /// same residual. (True *direction* alternates 90/270 as that
    /// residual's sign flips tick to tick -- also expected, not a bug;
    /// meaningless anyway at near-zero speed.)
    ZeroTrueWindAcceleratingBoat,
}

impl Scenario {
    pub const ALL: &'static [Scenario] = &[Scenario::VaryingWind, Scenario::ZeroTrueWindAcceleratingBoat];

    pub fn name(&self) -> &'static str {
        match self {
            Scenario::VaryingWind => "varying-wind",
            Scenario::ZeroTrueWindAcceleratingBoat => "zero-true-wind-accelerating-boat",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|s| s.name() == name)
    }

    pub fn ticks(&self) -> Vec<ScenarioTick> {
        match self {
            Scenario::VaryingWind => varying_wind(),
            Scenario::ZeroTrueWindAcceleratingBoat => zero_true_wind_accelerating_boat(),
        }
    }
}

fn varying_wind() -> Vec<ScenarioTick> {
    (0..TICKS)
        .map(|i| {
            let speed = 6.0 + 3.0 * ((i as f32) * 0.4).sin();
            let direction = (i as f32) * (360.0 / TICKS as f32);
            ScenarioTick {
                apparent_speed_mps: speed,
                apparent_direction_deg: direction,
                boat_sog_mps: 3.0 * MPS_PER_KNOT, // 3 kn, constant
                boat_cog_deg: 45.0,
            }
        })
        .collect()
}

fn zero_true_wind_accelerating_boat() -> Vec<ScenarioTick> {
    (0..TICKS)
        .map(|i| {
            // Ramps ~1.0 -> ~8.5 m/s (roughly 2-16 kn) over the run.
            let boat_speed_mps = 1.0 + 0.5 * (i as f32);
            ScenarioTick {
                apparent_speed_mps: boat_speed_mps,
                apparent_direction_deg: 0.0,
                boat_sog_mps: boat_speed_mps,
                boat_cog_deg: 90.0, // constant heading -- "same direction" throughout
            }
        })
        .collect()
}
