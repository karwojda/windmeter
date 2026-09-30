# Project: windmeter

## Problem

Sailors need accurate apparent and true wind readings while underway.
Off-the-shelf options are either expensive proprietary masthead units or
offer no local data logging for later analysis. The user wants their own
instrument for their sailboat: sense apparent wind, combine it with GPS to
compute true wind, log both locally, and share the result with the boat's
existing NMEA-capable instruments.

## Approach

A boat-mounted embedded device (STM32 microcontroller, Rust/Embassy
firmware) reads a wind sensor (cup-and-vane initially, ultrasonic planned
as a follow-on) and an onboard GPS module, computes true wind, logs
locally, and broadcasts on the boat's NMEA instrument bus. Requirements
and architecture are captured formally in SysML v2 under
`requirements/model/`; WhittleSpec's `requirements.md`/`plan.md` serve as
the lightweight decision/review surface over that model rather than
duplicating it.

## Success Looks Like

- Device installed on the boat, reliably reporting apparent and true wind
  speed/direction.
- Wind and GPS data logged locally and usable for post-sail analysis.
- Computed wind data visible on existing NMEA-capable instruments (e.g. a
  chartplotter).
- The wind sensor can be swapped from cup-and-vane to ultrasonic without
  redesigning the rest of the device.

## Boundaries

- NOT (v1): multi-boat/regatta features (fleet tracking, race committee,
  web viewer) explored in the earlier `sailing_meteo_station` project --
  a possible future direction, not ruled out, but not in this project's
  current scope.
- NOT (v1): ultrasonic wind sensor hardware -- the architecture supports
  it, but v1 ships with cup-and-vane.
- Not yet decided: boat heading source beyond GPS course-over-ground
  (velocity-over-ground is a first-iteration simplification, accurate
  only absent current/leeway). NMEA2000 over CAN and SD-card local
  storage are both decided (see `requirements/model/requirements.sysml`
  and `PROJECT_DESCRIPTION.md`).

## Tier

Operational -- a personal instrument intended for real, ongoing use
aboard the boat, not a throwaway experiment. Not tied to a business or
strategy initiative.

## Issue Tracker

Not yet. Work ledger is `local` (see `WHITTLESPEC.md`) -- tracked in each
spec's `tasks.md`.

## Related Projects

- `sailing_meteo_station` (`/mnt/c/git/sailing_meteo_station`) -- earlier,
  broader exploration of a multi-boat regatta ecosystem (boat units,
  buoys, race committee, web viewer). windmeter reuses its SysML domain
  vocabulary and patterns, narrowed to a single boat's own instrument. May
  reconnect to it later if the multi-boat direction is picked back up.

## Spec Seeds

| Feature       | Seed                         | Status |
| ------------- | ----------------------------- | ------ |
| windmeter-v1  | specs/windmeter-v1/seed.md    | seed   |

## History

- 2026-09-18: Created via `/ws.init`, after initial SysML
  requirements/architecture drafting (`requirements/model/{domain,system,requirements}.sysml`)
  and `WHITTLESPEC.md` binding setup (verification, durability, work-ledger).
