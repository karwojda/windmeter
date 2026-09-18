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

## SysML Conventions

`part def` / `item def` / `attribute def` / `requirement def` names are
PascalCase; usages (part/item/requirement instances, including
requirement short IDs like `<'REQ-001'>`) are camelCase -- the `sysml`
CLI's style linter (`STYL002`) enforces this. Requirement short IDs use
`REQ-NNN`.
