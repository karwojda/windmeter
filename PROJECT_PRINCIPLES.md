# Project Principles: windmeter

Non-obvious constraints worth knowing before touching this project.
Process (how SDD is run) lives in `ws._meta`; this file is project-specific.

## Requirements & Architecture Format

Formal requirements and architecture live in SysML v2 textual notation
under `requirements/model/` (`domain.sysml`, `system.sysml`,
`requirements.sysml`), validated with `sysml -validate requirements/model`
(the `sysml`/`sysml-lsp` CLI, already on PATH). WhittleSpec's
`requirements.md`/`plan.md` are the decision/review surface over that
model -- they point at the relevant `.sysml` package/requirement rather
than restating it.

## Platform & Language

- Firmware: Rust, `no_std` / `no_main`, Embassy async executor.
- Target: STM32H723ZG. May change later -- `rust-toolchain.toml` lists the
  full set of supported embedded targets.
- Embedded-only dependencies sit behind the `embedded` Cargo feature (on
  by default) so host-side logic can be unit-tested without hardware.

## Verification

Full binding in `WHITTLESPEC.md`. Host gate: `cargo test --no-default-features`.
Embedded target must also keep building:
`cargo build --target thumbv7em-none-eabihf`. On-target hardware tests
(`tests/wind_meter.rs`, via `probe-rs run` on a real STM32H723ZG) need
attached hardware and are run manually, not part of the automated gate.

## CI

`.github/workflows/ci.yaml` runs on every push to `main` and every PR:
builds the `Dockerfile` image (Rust + the `thumbv7em-none-eabihf` target +
`sysml`/`sysml-lsp` + `cargo-llvm-cov`; cached via GHA buildx cache), then
runs `ci/verify.sh` inside it -- SysML model validation, host tests,
coverage (gated at 85% lines, see the script), and the embedded build.
The lcov report is uploaded as a workflow artifact.

Deliberately excluded from CI: `cargo-mutants` (a full-crate run takes
tens of minutes -- too expensive per push/PR) and on-target hardware
tests (need real hardware, per Verification above). Run both locally.

`ci/verify.sh` is meant to double as the local one-shot check -- same
script, same order, whether run directly (if `sysml`/`cargo-llvm-cov`
are installed) or via `docker run ... ./ci/verify.sh` for full parity
with CI.

## Test Quality: Coverage and Mutation Testing

These are quality-control checks, not part of the per-task completion
gate (`WHITTLESPEC.md`'s `verification` binding stays
`cargo test --no-default-features` alone) -- run periodically/in CI to
catch coverage gaps and weak assertions, not on every task.

**Coverage** -- `cargo-llvm-cov` (LLVM source-based instrumentation;
chosen over `cargo-tarpaulin` for more accurate region/branch coverage
and cross-platform support). Requires the `llvm-tools` rustup component
(`rustup component add llvm-tools`).

```sh
cargo llvm-cov --no-default-features --summary-only   # quick check
cargo llvm-cov --no-default-features --html            # target/llvm-cov/html/index.html
cargo llvm-cov --no-default-features --fail-under-lines 85   # matches ci/verify.sh's gate
```

**Mutation testing** -- `cargo-mutants`. Config lives in
`.cargo/mutants.toml`: excludes `src/bin/*.rs` (requires the `embedded`
feature + the STM32 target, can't build on host) and passes
`--no-default-features` to every build/test it runs, matching the
verification binding.

```sh
cargo mutants                          # full run (can take a while; scales with test suite runtime x mutant count)
cargo mutants -f "src/wind_compute.rs" # scope to one file while iterating
```

A "missed" mutant means some code change didn't make any test fail --
either dead code, or a real gap in what the tests actually assert. Not
every miss is worth chasing (e.g. a missed `Debug` derive mutant is
usually noise); use judgment.

## Simulating and Visualizing a Log Without Hardware

`cargo run --no-default-features --features sim --bin simulate -- [--scenario
NAME] [output.csv]` runs the real sense -> compute -> log pipeline against
synthetic data (no hardware needed) and writes a CSV in the same format a
real SD card log would have (`LogRecord::CSV_HEADER`, `src/logger.rs`).
`--list-scenarios` prints the available names (`src/bin/simulate/scenarios.rs`
-- add a scenario there; `main.rs`'s pipeline wiring is scenario-agnostic).
Gated behind the `sim` feature, which nothing else enables, so it can't leak
into the embedded build or `cargo test`/`cargo llvm-cov` (see `tasks.md`
Task 6).

`scripts/visualize_log.py <log.csv>` plots either that simulated output or
a real pulled-card log -- speed, direction, and GPS speed-over-ground each
in their own panel (different units, never share an axis). Needs
pandas + plotly, already in `.venv` (`.venv/bin/python scripts/visualize_log.py ...`).

## Booting the Firmware Under Emulation (Renode)

`renode/boot.resc` loads the real compiled ELF
(`cargo build --target thumbv7em-none-eabihf` first) onto an emulated
STM32H723ZG and runs it for a couple of virtual seconds -- proves the
image actually boots without needing hardware. `renode/stm32h723zg.repl`
is a thin overlay on Renode's bundled H7-family platform description
(deliberately not size/address-correcting every RAM region -- see its
header comment for why that's a documented non-issue for this level of
testing). Run from Renode's own install directory (its `i @platforms/...`
include paths are resolved relative to it):

```sh
/path/to/renode --disable-gui --console \
  -e 'i @/path/to/windmeter/renode/boot.resc' -e quit
```

Renode's bundled RTT support hooks SEGGER's own C function symbols,
which `defmt-rtt` (a from-scratch reimplementation) never calls, so
`renode/fault_probe.py` reads the standard RTT control block directly out
of RAM instead -- works for any RTT flavor, not just SEGGER's reference
implementation. It also hooks `HardFault` so a `panic-probe` panic (which
traps into a debugger breakpoint Renode has none attached for) captures
the exception frame instead of silently free-running past it. Decode
whatever it captured with the real defmt-print tool
(`cargo install defmt-print`):

```sh
defmt-print -e target/thumbv7em-none-eabihf/debug/wind_meter \
  < /tmp/windmeter_rtt.bin
```

**CAN is wired and fully working**: `boot.resc` connects `fdcan1` to a
`CANHub` and `fault_probe.py`'s `probe_can` logs every frame it
transmits (id + raw bytes) to `/tmp/windmeter_can.log` -- this is real
confirmation that the FDCAN1 peripheral driver code produces correct
PGN 130306 bytes on a (virtual) bus, not just a unit-tested encoding
function. Still not a substitute for tasks.md Task 5a's actual point
(a real NMEA2000 bus/analyzer, or a real chartplotter showing the
data) -- see its own acceptance criteria.

**GPIO (anemometer pulses) and UART (GPS sentences) are not**, despite
real effort: `boot.resc` drives them (`gpioPortA OnGPIO`/`usart3
WriteLine`), and those calls demonstrably change the right hardware
state (confirmed separately by reading GPIOA's IDR and USART3's ISR
back), but the firmware never reacts to either -- EXTI0's handler is
never entered and USART3's RXNE flag never sets, even with
NVIC/EXTI/RTSR1/IMR1 all correctly configured per a register-level
check. This looks like a gap in how this Renode peripheral-model
combination propagates externally-injected stimulus through to
STM32-specific interrupt/DMA machinery, not a firmware bug -- left in
`boot.resc` as working groundwork for whoever revisits it.

Three real clock-configuration bugs were found and fixed along the
way (`wind_meter.rs`'s `Config::default()` left several peripherals on
a PLL-sourced mux with no PLL ever enabled) -- these would have bitten
real hardware too, not just Renode:
- ADC1 and SPI1 defaulted to a PLL2/PLL1 mux with nothing running it --
  routed both via the `per` mux from HSI instead (already running).
- FDCAN1 has no HSI/`per` bypass (only HSE or a PLL output) -- given a
  small dedicated PLL1 (Q output only, sysclk stays on HSI).

Also found: `logger_task`'s SD-card retry logic
(`embedded-sdmmc`'s `AcquireOpts { retries: 50 }`) blocks the entire
single-threaded embassy executor for several virtual seconds per tick
when no card responds -- every other task (wind sensing, GPS, CAN
output) stalls along with it. Real behavior, not a Renode artifact;
worth keeping in mind for Task 4a (a slow or faulty real card could
have the same effect, which REQ-005's negative AC doesn't currently
cover -- it only requires "doesn't crash", not "doesn't stall
everything else").

Known Renode-side gap, not a firmware bug: its `STM32H7_RCC` model logs
`Unhandled write ... Tags: ADCSEL`/`SPI123SEL`/`FDCANSEL` and doesn't
persist the bits those tags name -- confirmed by reading each register
back and seeing them never stuck, regardless of what was written.
`boot.resc` works around it with `AddBeforeReadDoubleWordHook`, forcing
reads of the two affected registers to return what the firmware's own
writes intended (captured from the warnings' logged values). Revisit if
a later Renode version fixes this and the register-level workaround is
no longer needed.

**Automated regression tests**: `renode/tests/*.robot` run the same
kind of checks as `boot.resc` (repl, RCC hooks, load ELF, drive
peripherals) as Robot Framework tests via Renode's own `renode-test`
runner -- `boot.robot` asserts no HardFault happened;
`can_output.robot` additionally connects a CAN hub and asserts a real
NMEA2000 (PGN 130306) frame with the right 29-bit ID actually left the
FDCAN1 driver. Both share machine bring-up via
`renode/tests/common.resource`'s `Boot Windmeter Firmware` keyword.
Needs Renode's bundled test dependencies installed once into a venv:

```sh
python3 -m venv /path/to/venv
/path/to/venv/bin/pip install -r /path/to/renode/tests/requirements.txt
```

Then, from Renode's own install directory (same path-resolution
reason as `boot.resc`):

```sh
source /path/to/venv/bin/activate
./renode-test /path/to/windmeter/renode/tests/*.robot
```

Detecting a fault this way needs care: comparing `sysbus.cpu PC`
against `HardFault_`'s address *after* `emulation RunFor` doesn't
work, since by the time PC is sampled, the handler has already
executed a few instructions past its own entry point and PC no longer
matches it exactly, even though execution is still inside it (found by
deliberately reintroducing the ADC1 clock-mux bug and watching the
naive version of this test still pass). `fault_probe.py`'s
`probe_hard_fault` takes an optional marker-file path instead, written
the moment the handler is *entered* (a hook, not a post-hoc PC
comparison) -- the test asserts that file doesn't exist. Verified both
ways: passes against the current (fixed) firmware, fails with the
right file-exists message when the ADC1 clock-mux fix is reverted.

## SysML Conventions

`part def` / `item def` / `attribute def` / `requirement def` names are
PascalCase; usages (part/item/requirement instances, including
requirement short IDs like `<'REQ-001'>`) are camelCase -- the `sysml`
CLI's style linter (`STYL002`) enforces this. Requirement short IDs use
`REQ-NNN`.
