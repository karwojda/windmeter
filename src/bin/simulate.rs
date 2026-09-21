//! Host-only simulation: runs the sense -> compute -> log pipeline
//! (`wind_sensor` -> `gps` -> `wind_compute` -> `logger`) against
//! synthetic data, with no hardware at all. An opt-in dev tool for
//! sanity-checking the pipeline by eye before hardware is in hand -- not
//! part of the test suite (those already cover this logic in isolation;
//! this exercises it wired together, end to end). See `tasks.md` Task 6.
//!
//! Run: `cargo run --no-default-features --features sim --bin simulate
//! [output-file]` (defaults to `simulate_log.csv` in the current
//! directory).

use std::env;
use std::fs::File;
use std::io::Write as _;

use wind_meter::gps::GpsFixTracker;
use wind_meter::logger::LogRecord;
use wind_meter::wind_compute::compute_true_wind;
use wind_meter::wind_sensor::{ApparentWindSampler, WindCalibration};

const SAMPLE_PERIOD_S: f32 = 1.0;
const TICKS: usize = 16;

fn main() {
    let out_path = env::args().nth(1).unwrap_or_else(|| "simulate_log.csv".to_string());
    let mut file = File::create(&out_path).expect("create output file");
    writeln!(file, "{}", LogRecord::CSV_HEADER).expect("write header");

    let calibration = WindCalibration::davis_6410_defaults();
    let mut wind_sampler = ApparentWindSampler::new(calibration);
    let mut gps_tracker = GpsFixTracker::new(5.0);

    println!("Simulating {TICKS} sample ticks (no hardware) -> {out_path}\n");
    println!("{}", LogRecord::CSV_HEADER);

    for i in 0..TICKS {
        // Synthetic apparent wind: speed drifts 3..9 m/s, direction
        // sweeps a full rotation over the run -- reverses the same
        // formulas ApparentWindSampler uses, so this is real sensor-shape
        // input (pulse count + raw ADC code), not a shortcut around it.
        let target_speed_mps = 6.0 + 3.0 * ((i as f32) * 0.4).sin();
        let target_direction_deg = (i as f32) * (360.0 / TICKS as f32);
        let pulse_count = (target_speed_mps / calibration.mps_per_hz * SAMPLE_PERIOD_S).round() as u32;
        let direction_adc = (target_direction_deg / 360.0 * calibration.adc_max as f32).round() as u16;
        let apparent = wind_sampler.sample(pulse_count, direction_adc, SAMPLE_PERIOD_S);

        // Synthetic GPS: steady boat velocity (3 kn @ 45 deg COG) fed as
        // a real NMEA0183 RMC sentence through the actual parser -- not
        // a GpsFix built by hand -- so Task 2's parsing is exercised too,
        // not just Task 3's arithmetic.
        let sentence = rmc_sentence(3.0, 45.0);
        gps_tracker.feed_bytes(sentence.as_bytes());
        let gps = gps_tracker.sample(SAMPLE_PERIOD_S);

        let true_wind = compute_true_wind(apparent, gps);

        let record = LogRecord {
            timestamp_ms: (i as u64) * (SAMPLE_PERIOD_S * 1000.0) as u64,
            apparent,
            true_wind,
            gps,
        };

        let mut buf = [0u8; 160];
        let line = record.format_csv(&mut buf).expect("record fits");
        println!("{line}");
        writeln!(file, "{line}").expect("write record");
    }

    println!("\nWrote {TICKS} rows to {out_path} -- open it (or scroll up) to inspect by hand.");
}

/// Builds a valid NMEA0183 RMC sentence with a real, computed checksum --
/// hand-typing one was exactly how a bug slipped into gps.rs's own tests
/// earlier in this project; computing it programmatically here avoids
/// repeating that mistake.
fn rmc_sentence(sog_knots: f32, cog_deg: f32) -> String {
    let body = format!("GPRMC,125504.049,A,5542.2389,N,03741.6063,E,{sog_knots:.2},{cog_deg:.2},200906,,,A");
    let checksum = body.bytes().fold(0u8, |acc, b| acc ^ b);
    format!("${body}*{checksum:02X}\r\n")
}
