#!/bin/sh
# Full verification pass: SysML model validation, host tests, coverage,
# and the embedded (STM32H723ZG) build. Run inside the Dockerfile's
# image (what CI does -- see .github/workflows/ci.yaml) or directly on a
# machine that already has sysml/cargo-llvm-cov installed.
#
# Does NOT run cargo-mutants -- see PROJECT_PRINCIPLES.md § Test Quality.
set -eu

# Below this, coverage regressing is treated as a real problem, not
# noise. Set well under the current ~99% so adding real (currently
# host-untestable, e.g. more hardware-only code) surface doesn't break
# CI by itself -- revisit upward as the project grows.
COVERAGE_FAIL_UNDER_LINES=85

echo "== SysML model validation =="
sysml -validate requirements/model

echo
echo "== Host tests (cargo test --no-default-features) =="
cargo test --no-default-features

echo
echo "== Coverage (cargo llvm-cov --no-default-features) =="
cargo llvm-cov --no-default-features --fail-under-lines "$COVERAGE_FAIL_UNDER_LINES" --lcov --output-path lcov.info
cargo llvm-cov report --summary-only

echo
echo "== Embedded build: STM32H723ZG firmware image (cargo build --target thumbv7em-none-eabihf) =="
cargo build --target thumbv7em-none-eabihf

echo
echo "All checks passed."
