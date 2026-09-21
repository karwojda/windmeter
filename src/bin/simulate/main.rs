//! Host-only simulation: runs the sense -> compute -> log pipeline
//! (`wind_sensor` -> `gps` -> `wind_compute` -> `logger`) against
//! synthetic data, with no hardware at all. An opt-in dev tool for
//! sanity-checking the pipeline by eye before hardware is in hand -- not
//! part of the test suite (those already cover this logic in isolation;
//! this exercises it wired together, end to end). See `tasks.md` Task 6.
//!
//! Run: `cargo run --no-default-features --features sim --bin simulate --
//! [--scenario NAME] [output-file]` (scenario defaults to `varying-wind`;
//! output defaults to `simulate_log.csv`). `--list-scenarios` prints the
//! available names. What each scenario should produce, and why, is
//! documented on its `Scenario` variant in `scenarios.rs`.

mod scenarios;

use std::env;
use std::fs::File;
use std::io::Write as _;

use wind_meter::gps::GpsFixTracker;
use wind_meter::logger::LogRecord;
use wind_meter::wind_compute::compute_true_wind;
use wind_meter::wind_sensor::{ApparentWindSampler, WindCalibration};

use scenarios::Scenario;

const SAMPLE_PERIOD_S: f32 = 1.0;
const MPS_PER_KNOT: f32 = 0.514444;

struct Args {
    scenario: Scenario,
    out_path: String,
}

fn parse_args() -> Args {
    let mut scenario = Scenario::VaryingWind;
    let mut out_path = "simulate_log.csv".to_string();

    let args: Vec<String> = env::args().skip(1).collect();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--list-scenarios" => {
                for s in Scenario::ALL {
                    println!("{}", s.name());
                }
                std::process::exit(0);
            }
            "--scenario" => {
                let name = args.get(i + 1).unwrap_or_else(|| {
                    eprintln!("--scenario needs a name; see --list-scenarios");
                    std::process::exit(1);
                });
                scenario = Scenario::from_name(name).unwrap_or_else(|| {
                    let available: Vec<_> = Scenario::ALL.iter().map(|s| s.name()).collect();
                    eprintln!("unknown scenario '{name}'; available: {}", available.join(", "));
                    std::process::exit(1);
                });
                i += 2;
            }
            other => {
                out_path = other.to_string();
                i += 1;
            }
        }
    }

    Args { scenario, out_path }
}

fn main() {
    let args = parse_args();
    let ticks = args.scenario.ticks();

    let mut file = File::create(&args.out_path).expect("create output file");
    writeln!(file, "{}", LogRecord::CSV_HEADER).expect("write header");

    let calibration = WindCalibration::davis_6410_defaults();
    let mut wind_sampler = ApparentWindSampler::new(calibration);
    let mut gps_tracker = GpsFixTracker::new(5.0);

    println!(
        "Simulating scenario '{}' ({} sample ticks, no hardware) -> {}\n",
        args.scenario.name(),
        ticks.len(),
        args.out_path
    );
    println!("{}", LogRecord::CSV_HEADER);

    for (i, tick) in ticks.iter().enumerate() {
        // Reverses the same formulas ApparentWindSampler uses, so this is
        // real sensor-shape input (pulse count + raw ADC code), not a
        // shortcut around it.
        let pulse_count = (tick.apparent_speed_mps / calibration.mps_per_hz * SAMPLE_PERIOD_S).round() as u32;
        let direction_adc = (tick.apparent_direction_deg / 360.0 * calibration.adc_max as f32).round() as u16;
        let apparent = wind_sampler.sample(pulse_count, direction_adc, SAMPLE_PERIOD_S);

        // Fed as a real NMEA0183 RMC sentence through the actual parser
        // -- not a GpsFix built by hand -- so Task 2's parsing is
        // exercised too, not just Task 3's arithmetic.
        let sentence = rmc_sentence(tick.boat_sog_mps / MPS_PER_KNOT, tick.boat_cog_deg);
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

    println!("\nWrote {} rows to {} -- open it (or scroll up) to inspect by hand.", ticks.len(), args.out_path);
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
