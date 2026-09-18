# Tasks: windmeter-v1

## Implementation Constraints

**Permitted**: Libraries: `embassy-stm32` (add the `can` feature), `embedded-sdmmc`, `nmea0183`, existing `moving_average`/`windmeter` modules. Pattern: Embassy async tasks wired by `embassy-sync` channels (per `plan.md`). Files: `src/`, `src/bin/wind_meter.rs`, `Cargo.toml`.

**Not permitted**: rewriting `moving_average.rs`/`windmeter.rs` from scratch (extend the existing stub, per plan's brownfield note); ultrasonic sensor work (out of scope this cycle); a full N2K stack dependency (`korri-n2k`) unless hand-rolled PGN encoding proves insufficient (plan.md's documented fallback only); introducing a BDD/Gherkin framework (none exists in this crate; see BDD decision on each task).

**Style**: verification per `WHITTLESPEC.md` -- `cargo test --no-default-features` (host) plus `cargo build --target thumbv7em-none-eabihf` (embedded target still compiles). Docs land incrementally in `docs/operation.md`, one section per task.

## Slice Ledger

| # | User-job (what the skipper DOES) | Surfaces touched | Distinct slice because... |
|---|---|---|---|
| 1 | Checks the wind sensor reads correctly before trusting it | sensor hardware, RTT debug log | Verifiable on its own -- calibration check a sailor would actually do |
| 2 | Checks the GPS gets a fix before trusting it | GPS hardware, RTT debug log | Independently verifiable; GPS acquisition has its own real failure mode (antenna view of sky) |
| 3 | Sees the actual (true) wind, not just what the sensor feels | RTT debug log | The product's core differentiator over a bare wind vane; new user-observable number |
| 4 | Reviews a day's sailing afterward, from a boat with no power outlet | SD card, battery | The walking skeleton's endpoint: log survives a real session, runs untethered |
| 5 | Sees wind data on the chartplotter already at the helm | NMEA2000 bus, chartplotter | Second slice: rides a materially riskier, unproven transport (FDCAN), isolated so it can't block 1-4 |

## Involvement Summary

| Task | Level | Focus | Rationale |
|---|---|---|---|
| 1 [checkpoint] | checkpoint | AC1 (real pulse-capture accuracy) | Plan-flagged calibration risk, established GPIO/timer pattern otherwise |
| 2 [checkpoint] | checkpoint | AC1 (real sentence set/baud) | Plan-flagged `[VERIFY IN WALKING SKELETON]` item |
| 3 [checkpoint] | checkpoint | AC1 (arithmetic vs. `domain.sysml`) | Core formula is documented/understood, but is the product's central claim |
| 4 [checkpoint] | checkpoint | AC2 (storage fault must not crash) | Device runs unattended at sea; a crash is worse than a lost record |
| 5 [pair] | pair | AC1 (feasibility gate + fallback), AC3 (stale-suppression) | Highest novelty (first FDCAN use, plan's single point of failure) and reaches a real nav instrument |

## AC Coverage

| requirements.md AC | Covered by |
|---|---|
| REQ-001 positive | Task 1 |
| REQ-001 negative (disconnected sensor) | Task 1 |
| REQ-004 (swappable sensor, architecture-level) | Task 1 |
| REQ-003 (onboard GPS fix) | Task 2 |
| REQ-002 positive | Task 3 |
| REQ-002 negative (invalid GPS -> invalid true wind) | Task 3 |
| REQ-005 positive (logging) | Task 4 |
| REQ-005 negative (storage fault) | Task 4 |
| REQ-008 (battery/dinghy operation) | Task 4 |
| REQ-006 positive (NMEA2000 output) | Task 5 |
| REQ-006 negative (stale suppression) | Task 5 |

## Walking Skeleton

### Task 1 [checkpoint]: See live apparent wind readings

- [~] **Goal**: Cup-and-vane sensor produces live apparent wind (speed + direction), printed over RTT; disconnected sensor is flagged invalid, not stale.
- **Focus**: AC 1 -- does the pulse-capture/debounce actually hold up against a real spinning anemometer, not just a bench signal generator.
- **Touches**: `src/wind_sensor.rs` (new: `WindSensor` trait + `CupAndVaneWindSensor`), `src/windmeter.rs` (extend `Wind<N>`'s stub `filter_speed`/`filter_direction` to use real `moving_average::MovingAverage`), `src/bin/wind_meter.rs` (spawn the task).
- **Depends on**: None
- **Parallel**: yes (alongside Task 2)
- **Acceptance**:
  1. Powered on with the anemometer/vane connected, spinning the cups by hand produces changing speed/direction values in the RTT log within ~1s. -- **Demo**: `probe-rs run`, spin the cups by hand, watch values change in the terminal.
  2. Sensor disconnected/unresponsive -> reading flagged invalid rather than a frozen last value silently reported as current. -- **Demo**: unplug the sensor mid-run, see the "invalid" flag replace the last number, not a stuck reading.
  3. `WindSensor` trait exists; `CupAndVaneWindSensor` implements it (groundwork for REQ-004 -- checked by reading the code, not a runtime behavior).
- **Reachability**:
  - **Surface**: physical sensor + RTT debug log via `probe-rs run` (this project's dev-time observation channel; no product UI exists).
  - **Boss demo**: "Does the wind sensor work?" -> connects sensor+board, runs `probe-rs run`, spins the cups, reads live numbers off the terminal.
  - **Three regret scenarios**: (1) No disconnect check -> a loose wire silently reports a frozen "last good" speed as live. (2) No `WindSensor` trait -> swapping to ultrasonic later means touching this code again, breaking REQ-004. (3) Cross-cut: skip this task -> no apparent-wind number exists at all; Tasks 3-4 have nothing to compute from or log.
- **Tests green after**: `cargo test --no-default-features` + hardware demo above.
- **BDD decision**: do not use -- not applicable: no BDD/Gherkin framework in this embedded crate; `cargo test` plus the physical hardware demo serve the same red-to-green role.
- **Documentation**: `docs/operation.md` (wind sensor section).

### Task 2 [checkpoint]: See a live GPS fix

- [ ] **Goal**: Onboard GPS UART stream parsed into a live fix (position, speed-over-ground, course-over-ground), printed over RTT; no/stale fix is flagged invalid.
- **Focus**: AC 1 -- the GPS module's actual NMEA0183 sentence set and baud rate, flagged `[VERIFY IN WALKING SKELETON]` in `plan.md` (may differ from assumption).
- **Touches**: `src/gps.rs` (new: UART task + `nmea0183` crate), `src/bin/wind_meter.rs` (spawn the task).
- **Depends on**: None
- **Parallel**: yes (alongside Task 1)
- **Acceptance**:
  1. GPS module outdoors with sky view, live position/SOG/COG values appear in the RTT log within the module's normal acquisition time. -- **Demo**: `probe-rs run` outdoors, watch a fix appear in the terminal.
  2. No/stale fix -> flagged invalid rather than reporting a frozen last position. -- **Demo**: run indoors/antenna covered, see "no fix" rather than a stuck value.
- **Reachability**:
  - **Surface**: physical GPS module + RTT debug log (Task 1's channel).
  - **Boss demo**: "Does the GPS work?" -> powers on outdoors, runs `probe-rs run`, watches a fix appear.
  - **Three regret scenarios**: (1) No invalid-fix flagging -> Task 3 silently computes true wind from a stale position, producing plausible-looking garbage. (2) Only tested against synthetic sentences, never the real module -> the flagged sentence-set/baud risk goes unverified. (3) Cross-cut: skip this task -> Task 3 has no velocity input, only apparent wind -- the core differentiator never ships.
- **Tests green after**: `cargo test --no-default-features` (NMEA0183 parsing on captured samples) + hardware demo.
- **BDD decision**: do not use -- not applicable (same reason as Task 1).
- **Documentation**: `docs/operation.md` (GPS section).

### Task 3 [checkpoint]: See true wind computed live

- [ ] **Goal**: Combine live apparent wind (Task 1) and GPS fix (Task 2) into true wind via the `domain.sysml` formula, printed over RTT; either input invalid -> true wind flagged invalid, not computed from bad data.
- **Focus**: AC 1 -- arithmetic correctness against `domain.sysml`'s `trueWind = apparentWind - velocityOverGround`, spot-checked against known cases.
- **Touches**: `src/wind_compute.rs` (new), `src/bin/wind_meter.rs` (wire the `embassy-sync` channels from Tasks 1-2).
- **Depends on**: Task 1, Task 2
- **Acceptance**:
  1. With sensor and GPS both live and valid, printed true wind is consistent with apparent wind and boat velocity for a couple of known-by-hand cases. -- **Demo**: RTT log shows true wind alongside apparent wind and GPS SOG/COG; check the arithmetic by hand.
  2. Apparent wind invalid OR GPS fix invalid -> true wind flagged invalid, not computed from the bad input. -- **Demo**: unplug the sensor, or block the GPS antenna; true wind shows "invalid", not a number.
- **Reachability**:
  - **Surface**: RTT debug log (Tasks 1-2's channel).
  - **Boss demo**: "Show me the actual wind, not just what the sensor feels" -> powers on with both live, reads true wind off the log, compares by feel to the wind outside.
  - **Three regret scenarios**: (1) No invalid-propagation -> a GPS dropout produces a wrong true-wind number that looks legitimate. (2) Cross-cut: ship Tasks 1-2 without this -> the device only reports apparent wind -- REQ-002's whole point and the product's actual pitch over a bare wind vane. (3) Ship only tasks before this one -> same gap, contradicting the requirements' user story.
- **Tests green after**: `cargo test --no-default-features` (vector-subtraction, both invalid-input cases).
- **BDD decision**: do not use -- not applicable (same reason as Task 1).
- **Documentation**: `docs/operation.md` (true wind section).

**INTEGRATION MILESTONE**: After Task 3, verify the full sense-compute path live on the bench before moving to persistence/output.

### Task 4 [checkpoint]: Review a day's sailing afterward

- [ ] **Goal**: Append live apparent wind, true wind, and GPS records to a removable SD card as they're produced; storage faults don't crash the device; the whole loop runs on the unit's own battery for a full session. This is the plan's walking skeleton.
- **Focus**: AC 2 -- storage-fault handling must not crash a device meant to run unattended at sea.
- **Touches**: `src/logger.rs` (new: `embedded-sdmmc` over SPI), `src/bin/wind_meter.rs` (wire the task).
- **Depends on**: Task 1, Task 2, Task 3
- **Parallel**: yes (alongside Task 5, once this task's own dependencies are met)
- **Acceptance**:
  1. Device runs a real session (bench or on-water) powered only by its own battery; pulling the SD card afterward and reading it on a laptop shows a record per interval matching the RTT log during the run (REQ-005 + REQ-008 combined). -- **Demo**: run untethered on battery for 10+ minutes, pull the card, open the log file on a laptop.
  2. Storage full or card fault -> device keeps sensing/computing (RTT log keeps updating) rather than crashing; the fault is visible in the log. -- **Demo**: fill or pull the card mid-run, confirm the device doesn't hang or panic.
- **Reachability**:
  - **Surface**: the SD card itself, read on a laptop -- the actual v1 product artifact (no RTT/probe-rs needed, unlike Tasks 1-3).
  - **Boss demo**: "Go sailing, then show me what the wind did." -> runs on battery for a session, pulls the card at the dock, opens it on a laptop, sees the wind history.
  - **Three regret scenarios**: (1) No storage-fault handling -> a filled/glitchy card (a real marine-electronics failure) crashes the device mid-sail, losing the whole session. (2) Never tested on battery alone -> REQ-008 (the dinghy-with-no-power case) silently unverified. (3) Cross-cut: ship 1-3 without this -> the device computes wind live but never delivers "review afterward" or "no boat power needed" -- explicit v1 must-haves; the product is a bench demo, not what `requirements.md` committed to.
- **Tests green after**: `cargo test --no-default-features` (log-record formatting) + hardware demo (both criteria).
- **BDD decision**: do not use -- not applicable (same reason as Task 1).
- **Documentation**: `docs/operation.md` (logging + power section).

## Remaining Tasks

### Task 5 [pair]: See wind data on my chartplotter

- [ ] **Goal**: Confirm the board's CAN peripheral talks to a real NMEA2000 network, then transmit computed wind as the Wind Data PGN (130306) at a regular interval; if real-network CAN proves unreliable, fall back to NMEA0183 (`requirements.md` Decision 1's runner-up) instead; stale/invalid true wind is not sent.
- **Focus**: AC 1 (feasibility gate + fallback decision -- plan.md's flagged single point of failure for REQ-006) and AC 3 (stale-suppression -- wrong data on a real nav instrument is worse than none).
- **Touches**: `src/nmea2000.rs` (new: FDCAN config + PGN encoding, or NMEA0183 output module if the fallback triggers), `Cargo.toml` (add `embassy-stm32`'s `can` feature), `src/bin/wind_meter.rs` (wire the task).
- **Depends on**: Task 3
- **Parallel**: yes (alongside Task 4)
- **Acceptance**:
  1. [Feasibility gate] A raw CAN frame sent from the board is received by a real NMEA2000 device/analyzer on an actual N2K bus, not just self-loopback. If this fails, switch to the NMEA0183 fallback for the rest of this task instead of proceeding on CAN. -- **Demo**: connect to a real N2K bus (or a CAN analyzer), send a test frame, observe it arrive; record which path (NMEA2000 or fallback) was taken and why, in `plan.md`.
  2. With the device connected to a real NMEA2000 network (a chartplotter or N2K display), wind data appears on that instrument at a regular interval, matching the RTT-logged values. -- **Demo**: connect to a chartplotter/N2K display, watch wind speed/direction update on screen.
  3. No valid true wind currently available -> that message is omitted or marked invalid, not sent stale. -- **Demo**: disconnect the GPS mid-run, confirm the instrument shows "no data"/stops updating rather than freezing on a wrong value.
- **Reachability**:
  - **Surface**: the boat's own NMEA2000-connected instrument -- a sailor looks at their existing instruments, not at this device directly.
  - **Boss demo**: "Check the wind on your chartplotter." -> looks at their already-installed chartplotter's wind page, sees the new device's data appearing there like any other N2K wind sensor.
  - **Three regret scenarios**: (1) No stale-suppression -> a GPS dropout leaves a wrong true-wind number frozen on the user's real navigation display -- worse than showing nothing. (2) Skipped the real-network feasibility gate, tested only in loopback -> "works," then fails silently once wired into the boat's live bus. (3) Cross-cut: ship 1-4 without this -> wind computes and logs correctly but REQ-006 (an explicit v1 must-have) never ships -- a data-logger, not the integrated instrument `requirements.md` committed to.
- **Tests green after**: `cargo test --no-default-features` (PGN/frame encoding) + hardware demo on a real N2K instrument.
- **BDD decision**: do not use -- not applicable (same reason as Task 1).
- **Documentation**: `docs/operation.md` (NMEA2000 output section).

## Backlog (Deferred)

- Ultrasonic wind sensor implementation -- architecture supports it (`WindSensor` trait, Task 1); not built this cycle (`requirements.md` Out of Scope).
- Wireless display unit -- parked in `specs/_ideas.md`.
- Numeric accuracy/rate/runtime targets, battery chemistry/charging method -- deferred per `requirements.md` Out of Scope, revisit after real-world testing.
