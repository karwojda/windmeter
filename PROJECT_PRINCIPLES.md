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

`cargo run --no-default-features --features sim --bin simulate [output.csv]`
runs the real sense -> compute -> log pipeline against synthetic data (no
hardware needed) and writes a CSV in the same format a real SD card log
would have (`LogRecord::CSV_HEADER`, `src/logger.rs`). Gated behind the
`sim` feature, which nothing else enables, so it can't leak into the
embedded build or `cargo test`/`cargo llvm-cov` (see `tasks.md` Task 6).

`scripts/visualize_log.py <log.csv>` plots either that simulated output or
a real pulled-card log -- speed, direction, and GPS speed-over-ground each
in their own panel (different units, never share an axis). Needs
pandas + plotly, already in `.venv` (`.venv/bin/python scripts/visualize_log.py ...`).

## SysML Conventions

`part def` / `item def` / `attribute def` / `requirement def` names are
PascalCase; usages (part/item/requirement instances, including
requirement short IDs like `<'REQ-001'>`) are camelCase -- the `sysml`
CLI's style linter (`STYL002`) enforces this. Requirement short IDs use
`REQ-NNN`.
