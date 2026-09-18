## Verification
Binding: cargo test --no-default-features

Host-side unit/integration tests (this crate's `lib`/`tests` target with the
`embedded` feature off) are the completion gate. Full check also includes:

- `cargo build --target thumbv7em-none-eabihf` — the embedded target (default
  features on) must keep compiling for the STM32H723ZG.
- `sysml -validate requirements/model` — the SysML v2 requirements/architecture
  model must analyse with no errors whenever `requirements/model/*.sysml`
  changes.
- On-target hardware tests (`embedded-test` via `probe-rs run`, real
  STM32H723ZG board) exist under `tests/wind_meter.rs` but need attached
  hardware, so they are not part of the automated gate — run manually before
  shipping a hardware-affecting change.

## Durability
Binding: git commit; timing: per-task

## Work ledger
Binding: local

SDD task files (`tasks.md` per spec) are the identity/state store: stable
task numbers = identity, `[ ]`/`[x]`/`[~]` = state. No external tracker in
use yet.
