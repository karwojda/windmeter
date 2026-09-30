#!/bin/sh
# Runs renode/tests/*.robot -- hardware-behavior regression checks (no
# HardFault on boot, a real NMEA2000 frame leaves FDCAN1) against an
# emulated STM32H723ZG, no physical board needed. See
# PROJECT_PRINCIPLES.md § Booting the Firmware Under Emulation.
#
# Needs the embedded build to already exist (ci/verify.sh's last step,
# or `cargo build --target thumbv7em-none-eabihf` directly) -- this
# script doesn't build it itself, since it's meant to run as a
# follow-on step after ci/verify.sh, not a substitute for it.
set -eu

WINDMETER_ROOT="$(pwd)"

. /opt/renode-venv/bin/activate
renode-test --variable "WINDMETER_ROOT:${WINDMETER_ROOT}" "${WINDMETER_ROOT}"/renode/tests/*.robot
