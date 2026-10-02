# windmeter

[![CI](https://github.com/karwojda/windmeter/actions/workflows/ci.yaml/badge.svg?branch=main)](https://github.com/karwojda/windmeter/actions/workflows/ci.yaml)
[![codecov](https://codecov.io/gh/karwojda/windmeter/branch/main/graph/badge.svg)](https://codecov.io/gh/karwojda/windmeter)

A boat-mounted instrument that senses apparent wind, reads GPS, computes
true wind, logs both locally, and broadcasts the result on the boat's
NMEA2000 instrument bus -- a personal replacement for expensive
proprietary masthead units that don't log data for later analysis.

Firmware: Rust (`no_std`, Embassy async executor) targeting an STM32H723ZG
(NUCLEO-H723ZG devkit). Requirements and architecture are modeled formally
in SysML v2 (`requirements/model/`), not just written as prose.

## Read this first

**[PROJECT_DESCRIPTION.md](PROJECT_DESCRIPTION.md)** -- requirements,
system architecture and a full traceability matrix, generated straight
from the SysML model (regenerate after any model change; see
[PROJECT_PRINCIPLES.md](PROJECT_PRINCIPLES.md#generating-project_descriptionmd)).
It can't drift from `requirements/model/*.sysml` the way a hand-written
summary could, since it's the model rendered, not a copy of it.

## Status

The firmware's sense -> compute -> log -> output pipeline is fully
implemented and verified two ways without needing the physical device:

- **Host-side simulation** (`cargo run --features sim --bin simulate`) --
  the real sensing/compute/logging logic against synthetic pulse/ADC/NMEA
  input, no hardware at all. See [PROJECT_PRINCIPLES.md](PROJECT_PRINCIPLES.md#simulating-and-visualizing-a-log-without-hardware).
- **Renode emulation** (`renode/`) -- the real compiled firmware, booted
  and driven on an emulated STM32H723ZG. This reaches further than
  simulation can: it exercises the actual embedded peripheral drivers
  (GPIO/ADC/UART/FDCAN), not just the host-portable logic above them, and
  it's what actually caught three real clock-configuration bugs before
  hardware ever would have. See [PROJECT_PRINCIPLES.md](PROJECT_PRINCIPLES.md#booting-the-firmware-under-emulation-renode).

**Not yet done**: confirmation against real hardware -- a real Davis 6410
wind sensor, a real GPS module outdoors, a real SD card through a real
power cycle, a real NMEA2000 bus or analyzer. These are deliberately
separate, deferred tasks (`1a`/`2a`/`4a`/`5a` in
[specs/windmeter-v1/tasks.md](specs/windmeter-v1/tasks.md)), blocked on
the BOM (`specs/windmeter-v1/context/hardware-sourcing.md`) being
assembled -- simulation and emulation are both deliberately scoped as
*complements* to that hardware pass, not substitutes for it.

## Building and testing

```sh
cargo test --no-default-features              # host tests (no hardware/target needed)
cargo build --target thumbv7em-none-eabihf     # embedded build (the real firmware image)
```

`ci/verify.sh` runs the full local/CI gate (SysML model validation, host
tests, coverage, embedded build); `ci/renode-test.sh` additionally runs
the Renode-based hardware-behavior regression tests. Both run inside the
project's `Dockerfile` image in CI (`.github/workflows/ci.yaml`) on every
push/PR.

Coverage is host-side line coverage from `cargo llvm-cov
--no-default-features`, gated at 85% in `ci/verify.sh` and published to
Codecov for the badge above. It measures the host-testable logic only:
the `embedded`-feature driver code (`src/bin/wind_meter.rs` and the
hardware modules) never builds on the host, so it isn't in the number --
that code is exercised by the Renode tests instead.

## More

- [PROJECT_INTENT.md](PROJECT_INTENT.md) -- problem, approach, success
  criteria, what's explicitly out of scope.
- [PROJECT_PRINCIPLES.md](PROJECT_PRINCIPLES.md) -- non-obvious
  constraints: SysML conventions, the embedded/host feature split, test
  quality tooling, the Renode setup, CI.
- [WHITTLESPEC.md](WHITTLESPEC.md) -- this project's SDD process bindings
  (verification/durability/work-ledger).
- [specs/windmeter-v1/](specs/windmeter-v1/) -- the working spec:
  `requirements.md`, `plan.md`, `tasks.md` and their decision history.
