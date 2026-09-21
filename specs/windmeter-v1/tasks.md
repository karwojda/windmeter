# Tasks: windmeter-v1

## Implementation Constraints

**Permitted**: Libraries: `embassy-stm32` (FDCAN -- no separate `can` feature flag, discovered during Task 5: it compiles in automatically for this chip, unlike originally assumed), `embedded-sdmmc`, `nmea0183`, existing `moving_average`/`windmeter` modules. Pattern: Embassy async tasks wired by `embassy-sync` channels (per `plan.md`). Files: `src/`, `src/bin/wind_meter.rs`, `Cargo.toml`. `src/bin/simulate.rs` (Task 6) behind the `sim` Cargo feature -- host-only, uses `std`, must never be reachable from a default-feature or `--no-default-features` build (see its `required-features` in `Cargo.toml`).

**Not permitted**: rewriting `moving_average.rs`/`windmeter.rs` from scratch (extend the existing stub, per plan's brownfield note); ultrasonic sensor work (out of scope this cycle); a full N2K stack dependency (`korri-n2k`) unless hand-rolled PGN encoding proves insufficient (plan.md's documented fallback only); introducing a BDD/Gherkin framework (none exists in this crate; see BDD decision on each task).

**Style**: verification per `WHITTLESPEC.md` -- `cargo test --no-default-features` (host) plus `cargo build --target thumbv7em-none-eabihf` (embedded target still compiles). Docs land incrementally in `docs/operation.md`, one section per task.

**2026-09-21 restructuring (via `/ws.refine`)**: every task below that had a hardware-only acceptance criterion has been split: the original task now covers what's verifiable via unit tests and/or Task 6's simulation (`cargo run --no-default-features --features sim --bin simulate`), and a new letter-suffixed sibling task (`1a`, `2a`, `4a`, `5a`) carries the real-hardware confirmation, deferred until hardware is in hand. Per the stable-numbering rule, no existing task was renumbered. Task 3 has no hardware-only sibling: it's pure arithmetic over Tasks 1/2's outputs, with no hardware risk of its own beyond what `1a`/`2a` already cover.

## Slice Ledger

| # | User-job (what the skipper DOES) | Surfaces touched | Distinct slice because... |
|---|---|---|---|
| 1 | Checks the wind sensor reads correctly before trusting it | sensor hardware (1a) / simulated input (now) | Verifiable on its own -- calibration check a sailor would actually do |
| 2 | Checks the GPS gets a fix before trusting it | GPS hardware (2a) / simulated NMEA sentences (now) | Independently verifiable; GPS acquisition has its own real failure mode (antenna view of sky) |
| 3 | Sees the actual (true) wind, not just what the sensor feels | RTT debug log / simulation output | The product's core differentiator over a bare wind vane; new user-observable number |
| 4 | Reviews a day's sailing afterward, from a boat with no power outlet | SD card, battery (4a) | The walking skeleton's endpoint: log survives a real session, runs untethered |
| 5 | Sees wind data on the chartplotter already at the helm | NMEA2000 bus, chartplotter (5a) | Second slice: rides a materially riskier, unproven transport (FDCAN), isolated so it can't block 1-4 |
| 6 | Sanity-checks the whole sense-compute-log pipeline without owning any hardware yet | plain CSV file on this machine | New user-job (the user has no boat hardware yet) that none of 1-5 serve -- not a re-split of them |

`1a`/`2a`/`4a`/`5a` aren't new rows here: they're deferred hardware confirmation of the *same* slice as their parent, not new user-jobs -- see Involvement Summary instead.

## Involvement Summary

| Task | Level | Focus | Rationale |
|---|---|---|---|
| 1 [checkpoint] | checkpoint | AC1 (mock pulse/ADC input produces correct output) | Now simulation-verified; real-hardware confirmation moved to 1a |
| 1a [pair] | pair | Both ACs -- entirely hands-on | I cannot perform any part of this myself (no hardware access); pure physical verification |
| 2 [checkpoint] | checkpoint | AC1 (synthetic NMEA sentences parse correctly) | Now simulation-verified; real module's actual sentence set/baud still unconfirmed -- moved to 2a |
| 2a [pair] | pair | Both ACs -- entirely hands-on | Same as 1a |
| 3 [checkpoint] | checkpoint | AC1 (arithmetic vs. `domain.sysml`) | Unchanged -- already fully verifiable without hardware (pure function over Tasks 1/2's outputs) |
| 4 [checkpoint] | checkpoint | AC2 (storage fault must not crash) | Code/logic now simulation-verified (log format, error-path structure); the physical "runs on battery, survives a real session" claim moved to 4a |
| 4a [pair] | pair | Both ACs -- entirely hands-on | Same as 1a |
| 5 [checkpoint] | checkpoint | AC (PGN encoding correctness) | Unit-tested; the CAN-bus/real-instrument claims (the actual feasibility gate) moved to 5a |
| 5a [pair] | pair | Both ACs -- entirely hands-on, highest-risk (plan's flagged single point of failure) | Same as 1a |
| 6 [checkpoint] | checkpoint | Does the wired-together pipeline (not just isolated functions) produce sane output | Already done -- see its own Completion Evidence below |

## AC Coverage

| requirements.md AC | Covered by (logic/simulation) | Real-hardware confirmation |
|---|---|---|
| REQ-001 positive | Task 1 | Task 1a |
| REQ-001 negative (disconnected sensor) | Task 1 (unit-tested; disconnect is a synthetic condition, not a physical one -- no hardware gap here) | -- |
| REQ-004 (swappable sensor, architecture-level) | Task 1 (code review) | -- |
| REQ-003 (onboard GPS fix) | Task 2 | Task 2a |
| REQ-002 positive | Task 3 | (implied by 1a + 2a; no separate hardware AC) |
| REQ-002 negative (invalid GPS -> invalid true wind) | Task 3 (unit-tested) | -- |
| REQ-005 positive (logging) | Task 4 (format/logic) | Task 4a (real session, real card) |
| REQ-005 negative (storage fault) | Task 4 (code structure: `Result`, no panicking path) | Task 4a (real fault, e.g. pulling the card mid-write) |
| REQ-008 (battery/dinghy operation) | -- (inherently physical) | Task 4a |
| REQ-006 positive (NMEA2000 output) | Task 5 (PGN encoding, unit-tested) | Task 5a |
| REQ-006 negative (stale suppression) | Task 5 (unit-tested) | -- |

## Walking Skeleton

### Task 6 [checkpoint]: Sanity-check the pipeline without hardware

- [x] **Goal**: Wire `wind_sensor` -> `gps` -> `wind_compute` -> `logger` together against synthetic data, in a host-runnable binary, so Tasks 1-4's logic can be checked end to end before hardware is in hand.
- **Touches**: `src/bin/simulate.rs` (new), `Cargo.toml` (new `sim` feature + `simulate` `[[bin]]`, both gated so they can never be reached by the embedded build or the test/coverage commands).
- **Depends on**: Task 1, Task 2, Task 3, Task 4 (their code, not their hardware verification)
- **Acceptance**:
  1. Running the binary produces a plain CSV file plus the same rows on stdout: synthetic apparent wind (varying speed/direction, generated as pulse-count/ADC-code pairs -- the sensor's actual output shape, not a shortcut past it), a real NMEA0183 RMC sentence per tick (checksum computed programmatically) fed through the actual parser, computed true wind, all via `LogRecord::format_csv`. -- **Demo**: `cargo run --no-default-features --features sim --bin simulate /tmp/out.csv`, then `cat /tmp/out.csv`.
  2. The `sim` feature/`simulate` binary are unreachable from every other build path. -- **Demo**: `cargo test --no-default-features`, `cargo build --target thumbv7em-none-eabihf`, `cargo llvm-cov --no-default-features` -- all three unaffected.
- **Reachability**: N/A -- a dev tool, not a product surface; its output (the CSV) is what tasks 1/2/3/4 now cite as their non-hardware demo.
- **Tests green after**: N/A (this *is* the manual-inspection tool; its correctness rides on `wind_sensor`/`gps`/`wind_compute`/`logger`'s own unit tests, which it wires together but doesn't re-test).
- **BDD decision**: do not use -- not applicable: dev tool, no user-facing behavioral contract.
- **Documentation**: this file (Implementation Constraints) + `src/bin/simulate.rs`'s own doc comment.

**COMPLETION EVIDENCE** (already run, this session):
1. Production wiring: `simulate` bin calls `ApparentWindSampler`, `GpsFixTracker`, `compute_true_wind`, `LogRecord::format_csv` directly -- same functions the real tasks use, not reimplementations.
2. Demo: ran `cargo run --no-default-features --features sim --bin simulate /tmp/simulate_log.csv`; got 16 rows, apparent speed cycling ~3-9 m/s, direction sweeping a full rotation, true wind visibly differing from apparent (boat velocity subtracted), GPS fix constant at the fed 3 kn @ 45 deg (1.543 m/s -- checks out: 3 * 0.514444). File contents confirmed via `cat`.
3. Residual patterns: n/a, new file.
4. Test suite: `cargo test --no-default-features` unaffected (38/38 still pass); `cargo build --target thumbv7em-none-eabihf` unaffected (clean); `cargo llvm-cov --no-default-features` unaffected (99.60%, `simulate.rs` correctly excluded since coverage doesn't pass `--features sim`).
5. Persistence: committed (`f842194`).

### Task 1 [checkpoint]: See live apparent wind readings

- [x] **Goal**: Cup-and-vane sensor logic produces correct apparent wind (speed + direction) from raw sensor-shaped input (pulse count + ADC code); disconnected/stale input is flagged invalid, not stale. **Scope narrowed 2026-09-21**: verified against unit tests + Task 6's synthetic input; real-hardware confirmation is Task 1a, deferred.
- **Focus**: AC 1 -- does the pulse-capture/debounce math hold up across a full input range (Task 6 sweeps 3-9 m/s and a full direction rotation), not just a single fixed test value.
- **Touches**: `src/wind_sensor.rs` (`WindSensor` trait + `CupAndVaneWindSensor`), `src/windmeter.rs` (`Wind<N>` smoothing), `src/bin/wind_meter.rs` (spawns the real hardware task, unchanged).
- **Depends on**: None
- **Parallel**: yes (alongside Task 2)
- **Acceptance**:
  1. Given synthetic pulse-count/ADC-code input across a realistic range, `ApparentWindSampler` produces correctly-varying speed/direction values. -- **Demo**: `cargo run --no-default-features --features sim --bin simulate`, watch `apparent_speed_mps`/`apparent_dir_deg` change plausibly tick to tick; `cargo test --no-default-features -- wind_sensor` for the exact-value assertions.
  2. Sensor disconnected/unresponsive (sustained zero pulses) -> reading flagged invalid rather than a frozen last value. -- **Demo**: `cargo test --no-default-features -- flags_invalid_after_sustained_zero_pulses` (this condition is synthetic by nature -- "zero pulses" needs no physical sensor to produce, so there's no hardware gap here, unlike the AC 1 speed/direction math).
  3. `WindSensor` trait exists; `CupAndVaneWindSensor` implements it (groundwork for REQ-004 -- checked by reading the code, not a runtime behavior).
- **Reachability**:
  - **Surface**: `cargo run --bin simulate`'s stdout/CSV output (this task's surface until 1a; the physical sensor + RTT log is 1a's surface).
  - **Boss demo**: "Does the wind math work?" -> runs the simulate binary, watches speed/direction change plausibly across the synthetic sweep.
  - **Three regret scenarios**: (1) No disconnect check -> a loose wire would silently report a frozen "last good" speed as live (still true on real hardware -- unaffected by this scope narrowing). (2) No `WindSensor` trait -> swapping to ultrasonic later means touching this code again, breaking REQ-004. (3) Cross-cut: skip this task -> no apparent-wind number exists at all; Tasks 3-4/6 have nothing to compute from or log.
- **Tests green after**: `cargo test --no-default-features` + Task 6's simulation demo.
- **BDD decision**: do not use -- not applicable: no BDD/Gherkin framework in this embedded crate.
- **Documentation**: `docs/operation.md` (wind sensor section).

### Task 1a [pair]: Verify wind sensor against real hardware (deferred)

- [ ] **Goal**: Confirm Task 1's logic holds with a *real* Davis 6410 connected -- the one thing simulation can't prove (does the actual pulse-capture/debounce and ADC reading behave as the math assumes on real silicon with a real spinning sensor).
- **Touches**: none (verification only, no code changes expected unless it fails)
- **Depends on**: Task 1
- **Acceptance**:
  1. Powered on with the anemometer/vane connected, spinning the cups by hand produces changing speed/direction values in the RTT log within ~1s. -- **Demo**: `probe-rs run`, spin the cups by hand, watch values change in the terminal.
  2. Sensor disconnected mid-run (unplug it) -> reading flagged invalid rather than a stuck reading. -- **Demo**: unplug the sensor mid-run, see the "invalid" flag replace the last number.
- **Reachability**: N/A -- verification-only, no new code/user surface.
- **Tests green after**: N/A (hardware demo only).
- **BDD decision**: do not use -- not applicable.
- **Blocked on**: Davis 6410 + NUCLEO-H723ZG in hand (`hardware-sourcing.md`).

### Task 2 [checkpoint]: See a live GPS fix

- [x] **Goal**: NMEA0183 RMC sentences parse into a correct fix (position, SOG, COG); no/stale fix is flagged invalid. **Scope narrowed 2026-09-21**: verified against unit tests (captured real-format sentences) + Task 6 (sentences generated and fed through the same parser each tick); real GPS module confirmation is Task 2a, deferred.
- **Focus**: AC 1 -- parsing correctness; the module's *actual* sentence set/baud rate (still an open risk -- Task 2a).
- **Touches**: `src/gps.rs` (`GpsFixTracker` + `nmea0183` crate), `src/bin/wind_meter.rs` (spawns the real hardware task, unchanged).
- **Depends on**: None
- **Parallel**: yes (alongside Task 1)
- **Acceptance**:
  1. Valid RMC sentences (with a correct, programmatically-computed checksum) produce the fix they encode. -- **Demo**: `cargo test --no-default-features -- gps::tests::extracts_fix_from_rmc_sentence`; `cargo run --no-default-features --features sim --bin simulate` (Task 6 feeds a fresh sentence every tick).
  2. No/stale fix -> flagged invalid rather than reporting a frozen last position. -- **Demo**: `cargo test --no-default-features -- fix_ages_out_after_max_age` (synthetic elapsed-time condition, no hardware gap here).
- **Reachability**:
  - **Surface**: `cargo run --bin simulate`'s output (Task 2a's surface is the physical GPS module + RTT log).
  - **Boss demo**: "Does the sentence parsing work?" -> runs the simulate binary, sees a GPS fix on every row.
  - **Three regret scenarios**: (1) No invalid-fix flagging -> Task 3 would silently compute true wind from a stale position (still true, unaffected by this narrowing). (2) Only ever tested against sentences this project generated, never the real module's actual output -> the flagged sentence-set/baud risk stays unverified until 2a. (3) Cross-cut: skip this task -> Task 3 has no velocity input, only apparent wind.
- **Tests green after**: `cargo test --no-default-features` + Task 6's simulation demo.
- **BDD decision**: do not use -- not applicable.
- **Documentation**: `docs/operation.md` (GPS section).

### Task 2a [pair]: Verify GPS against real hardware (deferred)

- [ ] **Goal**: Confirm Task 2's parser handles the *actual* NEO-6M sentence set/baud rate (plan.md's flagged `[VERIFY IN WALKING SKELETON]` risk) -- this is the one AC in the sense/compute path that simulation genuinely cannot substitute for, since it's specifically about whether reality matches the assumption the parser was built against.
- **Touches**: none (verification only; `gps.rs` if the real sentence set/baud differs from assumed)
- **Depends on**: Task 2
- **Acceptance**:
  1. GPS module outdoors with sky view, live position/SOG/COG values appear in the RTT log within the module's normal acquisition time. -- **Demo**: `probe-rs run` outdoors, watch a fix appear.
  2. No/stale fix (run indoors/antenna covered) -> "no fix" rather than a stuck value. -- **Demo**: run indoors, confirm invalid, not frozen.
- **Reachability**: N/A -- verification-only.
- **Tests green after**: N/A (hardware demo only).
- **BDD decision**: do not use -- not applicable.
- **Blocked on**: NEO-6M module + NUCLEO-H723ZG in hand.

### Task 3 [checkpoint]: See true wind computed live

- [x] **Goal**: Combine apparent wind + GPS fix into true wind via the `domain.sysml` formula; either input invalid -> true wind flagged invalid, not computed from bad data. **Unchanged by this restructuring** -- pure arithmetic over Tasks 1/2's outputs, with no hardware risk of its own; 1a + 2a's real-hardware confirmation is sufficient, no separate `3a` needed.
- **Focus**: AC 1 -- arithmetic correctness against `domain.sysml`'s `trueWind = apparentWind - velocityOverGround`, spot-checked against hand-computed cases (including an off-axis case where the earlier zero-COG test cases couldn't distinguish `+` from `-`).
- **Touches**: `src/wind_compute.rs`, `src/bin/wind_meter.rs` (wires the `embassy-sync` channels).
- **Depends on**: Task 1, Task 2
- **Acceptance**:
  1. With valid apparent wind + GPS fix, computed true wind matches hand-computed expectations for known cases (stationary boat, dead-downwind, and an off-axis case). -- **Demo**: `cargo test --no-default-features -- wind_compute`; `cargo run --no-default-features --features sim --bin simulate` shows true wind tracking apparent wind's sweep, offset by the fixed boat velocity.
  2. Apparent wind invalid OR GPS fix invalid -> true wind flagged invalid, not computed from the bad input. -- **Demo**: `cargo test --no-default-features -- invalid_apparent_wind_yields_invalid_true_wind`.
- **Reachability**:
  - **Surface**: RTT debug log (real hardware) / `cargo run --bin simulate`'s output (now).
  - **Boss demo**: "Show me the actual wind, not just what the sensor feels" -> runs the simulate binary, reads true wind off a row, compares to apparent wind + the fixed simulated boat velocity by hand.
  - **Three regret scenarios**: (1) No invalid-propagation -> a GPS dropout would produce a wrong true-wind number that looks legitimate. (2) Cross-cut: ship Tasks 1-2 without this -> the device only reports apparent wind -- REQ-002's whole point. (3) Ship only tasks before this one -> same gap, contradicting the requirements' user story.
- **Tests green after**: `cargo test --no-default-features` (vector-subtraction, both invalid-input cases) + Task 6's simulation demo.
- **BDD decision**: do not use -- not applicable.
- **Documentation**: `docs/operation.md` (true wind section).

**INTEGRATION MILESTONE** (revised): Task 6's simulation run now stands in for "verify the full sense-compute path" without hardware; the *live*, on-bench version of this milestone is folded into Task 4a below (by the time you're bench-testing on battery, sense+compute are already running).

### Task 4 [checkpoint]: Review a day's sailing afterward

- [x] **Goal**: Format and append apparent wind, true wind, and GPS records; the per-record write path (`append`) doesn't crash on a storage fault (`Result`-returning, no panicking call). **Scope narrowed 2026-09-21**: log-record correctness verified via unit tests + Task 6 (writes the same format to a real plain-text file you can inspect by hand); the physical "runs on battery, survives a real session on a real SD card" claim is Task 4a, deferred -- simulation cannot substitute for a physical power/storage fact.
- **Focus**: AC 2 -- storage-fault handling must not crash; verified here as an architectural property (no `unwrap`/`panic` in the write path), not a runtime test, since injecting a real card fault needs real hardware (Task 4a).
- **Touches**: `src/logger.rs` (`embedded-sdmmc` over SPI, real hardware path unchanged), `src/bin/simulate.rs` (Task 6: same `LogRecord`/CSV format, `std::fs::File` instead of `embedded-sdmmc`).
- **Depends on**: Task 1, Task 2, Task 3
- **Acceptance**:
  1. Records format correctly as CSV, including the invalid/false-flags case. -- **Demo**: `cargo test --no-default-features -- logger`; `cargo run --no-default-features --features sim --bin simulate /tmp/out.csv && cat /tmp/out.csv` -- open it by hand, exactly as you'd inspect a pulled SD card's file.
  2. `DataLogger::append` (the per-record write path, called every sample tick) has no panicking call -- verified by grep, not just read-through: `sed -n '/fn append/,/^    }/p' src/logger.rs | grep -c 'unwrap\|panic!\|expect('` returns 0. Separately, and *not* covered by this AC: `DataLogger::new` (one-time boot-time SPI setup) does call `.expect()` -- a different risk (fails fast at boot if the SPI device can't be constructed, vs. crashing mid-session on a card fault), arguably acceptable but worth Task 4a noticing, not silently equating with "no panics anywhere in this file."
- **Reachability**:
  - **Surface**: a plain CSV file, written by `cargo run --bin simulate` (Task 4a's surface is the real SD card).
  - **Boss demo**: "Show me the log format before we even have a card." -> runs the simulate binary, opens the resulting CSV in a text editor/spreadsheet, sees the same columns a real pulled card would have.
  - **Three regret scenarios**: (1) A CSV formatting bug ships unnoticed because nobody looked at real output -> now directly inspectable via Task 6, closing that gap early. (2) Cross-cut: ship 1-3 without this -> wind computes live but nothing persists -- REQ-005's whole point. (3) Storage-fault code path never gets code-reviewed -> a future edit could silently introduce a panic; this AC exists specifically so that's checked.
- **Tests green after**: `cargo test --no-default-features` (log-record formatting) + Task 6's simulation demo.
- **BDD decision**: do not use -- not applicable.
- **Documentation**: `docs/operation.md` (logging + power section).

### Task 4a [pair]: Verify SD logging + battery power against real hardware (deferred)

- [ ] **Goal**: Confirm the physical claims Task 4's simulation can't reach: the device runs a real session on a real SD card, powered only by its own battery (REQ-008), and storage faults (a real one, not a hypothesized one) don't crash it. This is the plan's walking-skeleton endpoint.
- **Touches**: none (verification only)
- **Depends on**: Task 4
- **Acceptance**:
  1. Device runs a real session (bench or on-water) powered only by its own battery; pulling the SD card afterward and reading it on a laptop shows a record per interval matching the RTT log during the run. -- **Demo**: run untethered on battery for 10+ minutes, pull the card, open the log file on a laptop.
  2. Storage full or card fault -> device keeps sensing/computing rather than crashing. -- **Demo**: fill or pull the card mid-run, confirm the device doesn't hang or panic. This is the one AC in this whole task list flagged since planning as verified "by code inspection, not test" (Task 4's completion evidence) -- deliberately exercising it for real here, not skipping it because Task 4 already looked done on paper.
  3. Boots successfully with the real SD card/SPI wiring connected as intended. -- **Demo**: power on, confirm `DataLogger::new`'s `.expect("SD card SPI device")` doesn't fire; if it does, that's a real boot-time failure mode Task 4's simulation couldn't have caught (see Task 4's AC 2 note).
- **Reachability**: N/A -- verification-only.
- **Tests green after**: N/A (hardware demo only).
- **BDD decision**: do not use -- not applicable.
- **Blocked on**: full BOM assembled and powered (`hardware-sourcing.md`).

## Remaining Tasks

### Task 5 [checkpoint]: See wind data on my chartplotter

- [x] **Goal**: Encode computed true wind as the Wind Data PGN (130306) correctly. **Scope narrowed 2026-09-21**: PGN encoding verified via unit tests; the CAN-bus feasibility gate and real-instrument confirmation -- the actual point of REQ-006 -- is Task 5a, deferred. Simulation has nothing to offer here: there's no meaningful host-side stand-in for "does a real NMEA2000 bus accept this frame."
- **Focus**: AC (stale-suppression correctness -- unit-tested) and the byte-level encoding itself.
- **Touches**: `src/nmea2000.rs` (PGN encoding, unchanged), `src/bin/wind_meter.rs` (real hardware wiring, unchanged).
- **Depends on**: Task 3
- **Acceptance**:
  1. `encode_wind_data` produces correct speed/angle/reference bytes at PGN 130306's native resolution. -- **Demo**: `cargo test --no-default-features -- nmea2000::tests::encodes_speed_and_angle_at_pgn_resolution`.
  2. Invalid true wind -> "data not available" (0xFFFF fields), not a stale number. -- **Demo**: `cargo test --no-default-features -- invalid_true_wind_encodes_as_not_available`.
- **Reachability**:
  - **Surface**: none yet -- there is no observable product surface until 5a puts a frame on a real bus. This task's payload is the encoding logic alone.
  - **Boss demo**: N/A at this scope -- see 5a for "check the wind on your chartplotter."
  - **Three regret scenarios**: (1) Wrong byte layout ships unnoticed -> caught by the resolution/rounding unit tests already in place. (2) Stale suppression missing -> a GPS dropout would freeze a wrong number on a real display (verified here, independent of the bus working). (3) Cross-cut: without 5a, this encoding logic never gets its actual real-world proof -- REQ-006 stays unmet regardless of how correct the bytes are.
- **Tests green after**: `cargo test --no-default-features`.
- **BDD decision**: do not use -- not applicable.
- **Documentation**: `docs/operation.md` (NMEA2000 output section).

### Task 5a [pair]: Verify NMEA2000 against real hardware (deferred, highest risk)

- [ ] **Goal**: Confirm the board's CAN peripheral talks to a real NMEA2000 network, then confirm wind data appears on a real chartplotter/display. This is `plan.md`'s flagged single point of failure for REQ-006 -- if this fails, the documented fallback is NMEA0183 (`requirements.md` Decision 1's runner-up), not silently shipping without bus output.
- **Touches**: `src/nmea2000.rs` (only if the CAN feasibility gate fails and the NMEA0183 fallback is triggered)
- **Depends on**: Task 5
- **Acceptance**:
  1. [Feasibility gate] A raw CAN frame sent from the board is received by a real NMEA2000 device/analyzer on an actual N2K bus, not just self-loopback. If this fails, switch to the NMEA0183 fallback instead of proceeding on CAN. -- **Demo**: connect to a real N2K bus (or a CAN analyzer), send a test frame, observe it arrive; record which path was taken and why, in `plan.md`.
  2. With the device connected to a real NMEA2000 network, wind data appears on that instrument at a regular interval. -- **Demo**: connect to a chartplotter/N2K display, watch wind speed/direction update on screen.
- **Reachability**:
  - **Surface**: the boat's own NMEA2000-connected instrument.
  - **Boss demo**: "Check the wind on your chartplotter." -> looks at their already-installed chartplotter's wind page, sees this device's data appearing there like any other N2K wind sensor.
- **Tests green after**: N/A (hardware demo only).
- **BDD decision**: do not use -- not applicable.
- **Blocked on**: CAN transceiver (SN65HVD230) + a real NMEA2000 bus/analyzer to test against (`hardware-sourcing.md`).

## Backlog (Deferred)

- Ultrasonic wind sensor implementation -- architecture supports it (`WindSensor` trait, Task 1); not built this cycle (`requirements.md` Out of Scope).
- Wireless display unit -- parked in `specs/_ideas.md`.
- Numeric accuracy/rate/runtime targets, battery chemistry/charging method -- deferred per `requirements.md` Out of Scope, revisit after real-world testing.
- Tasks 1a, 2a, 4a, 5a -- all blocked on hardware being in hand (`hardware-sourcing.md`); work through them in that order (1a/2a first -- cheapest to unblock and lowest risk; 5a last -- highest risk, has a documented fallback if it fails).
