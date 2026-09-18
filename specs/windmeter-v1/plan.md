# Plan: windmeter-v1

## Approach

Five Embassy async tasks on the STM32H723ZG, wired by `embassy-sync` channels: read the cup-and-vane sensor, read the onboard GPS (NMEA0183/UART), compute true wind, append to the SD-card log, encode/transmit NMEA2000 wind PGNs. Each task maps to a `system.sysml` part, keeping architecture and module boundaries in lockstep.

NMEA2000 is transmit-only (REQ-006 covers *computed wind*, not re-publishing GPS) -- hand-roll the one PGN needed (Wind Data 130306) on `embassy-stm32`'s FDCAN driver rather than pull in a full N2K stack. `korri-n2k` (no_std, Embassy-friendly) is a fallback if PGN encoding grows.

## Walking Skeleton

**Sensor and GPS connected, unit running on its own battery: pull the SD card after a sail and see logged apparent + true wind matching what happened on the water.**

NMEA2000 output is a second slice, sequenced after this: it rides the newer, less-proven FDCAN/N2K path, so isolating it keeps that risk off the sense-compute-log path everything else depends on.

**Build first to validate:** anemometer pulse capture, GPS module's real sentence set/baud, SD card write survives power loss and reads back on a laptop.

## Components

| Component | Purpose | Integrates With |
|---|---|---|
| `WindSensorTask` | Pulse count -> speed, ADC/encoder -> direction | Extends existing `windmeter::Wind<N>` (stub `filter_*` -> real `moving_average::MovingAverage`). Implements a `WindSensor` trait so `UltrasonicWindSensor` (REQ-004) drops in later. |
| `GpsTask` | Parse GPS UART stream into fixes | `nmea0183` crate |
| `WindComputeTask` | apparent wind + GPS velocity -> true wind (`domain.sysml`) | `embassy-sync` channels from the two tasks above |
| `LoggerTask` | Append wind+GPS records to SD card | `embedded-sdmmc` over SPI |
| `Nmea2000Task` | Encode wind PGN, transmit on CAN | `embassy-stm32` FDCAN (`can` feature, not yet enabled) |

`src/bin/wind_meter.rs` (currently an unrelated LED/PWM demo) becomes the real `main` that spawns these tasks.

## Data Structures

`WindReading { speed, direction, ts }`, `GpsFix { position, sog, cog, valid, ts }`, `TrueWind { speed, direction, ts }` -- host-testable, following `moving_average`'s pattern so `cargo test --no-default-features` covers the compute logic.

## Interfaces

`trait WindSensor { fn read(&mut self) -> WindReading; }` -- mirrors the `WindSensor` part def in `system.sysml`; `CupAndVane` implements it now, `Ultrasonic` later.

## Dependencies

- **Requires**: `embassy-stm32` `can` feature, `embedded-sdmmc`, `nmea0183` (all new).
- **Affects**: nothing yet (greenfield).

## Risks

| Risk | Impact | Mitigation |
|---|---|---|
| FDCAN/H7 driver immaturity (single point of failure for REQ-006) | NMEA2000 slice blocked/unreliable | Own slice, spiked before the rest of `Nmea2000Task`; if the spike fails, fall back to NMEA0183 (Decision 1's rejected alternative, still a credible path) rather than shipping without bus output |
| Anemometer pulse rate exceeds capture resolution at high wind | Speed reading saturates/wrong | Bench-verify with a signal generator against a reference anemometer |
| SD card corruption/wear at sea | Log data loss | Append-only writes, periodic flush; REQ-005 already requires the device keep running on storage fault |
| GPS SOG/COG standing in for boat velocity | True wind wrong under current/leeway | Accepted v1 limitation (`requirements.md`), not a defect |

## Open Questions

- [VERIFY IN WALKING SKELETON: GPS module's real NMEA0183 sentence set/baud; FDCAN reliability against a real N2K network]
- [ASSUMPTION: `Nmea2000Task` transmits wind PGN(s) only, not GPS position/SOG/COG -- the boat's other instruments already source that]
