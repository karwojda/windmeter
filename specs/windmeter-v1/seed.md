# Seed: windmeter-v1

**Status**: seed (not yet activated for SDD)
**Created**: 2026-09-18
**Source**: Project kickoff conversation -- scoped through direct
            discussion (sensor choice, GPS source, output/logging) and an
            initial SysML draft of the domain/architecture/requirements.
**Related**: `requirements/model/{domain,system,requirements}.sysml`

## Discovery

The user wants a working wind instrument for their own sailboat: sense
apparent wind, use an onboard GPS to derive true wind, log both locally,
and put the result on the boat's NMEA bus for other instruments. First
hardware iteration uses a cup-and-vane sensor; an ultrasonic sensor is a
planned follow-on, not part of this slice.

## Present Understanding

A `WindMeterUnit` (see `system.sysml`) composed of a `WindSensor`
(specialized by `CupAndVaneWindSensor` for v1), a `GpsReceiver`, a
`ProcessingUnit` (STM32H723ZG, Rust/Embassy), a `DataLogger`, and an
`NmeaOutput`. True wind is derived as apparent wind minus the boat's
GPS-derived velocity-over-ground (`domain.sysml`). Seven requirements
already drafted and `satisfy`'d against `windMeterV1` in
`requirements.sysml` (REQ-001..REQ-007): sense apparent wind, compute true
wind, onboard GPS, swappable wind sensor, local logging, NMEA output,
STM32/Rust platform.

## Open Questions

**For requirements / decision:**

- NMEA bus variant: NMEA0183 (serial) vs NMEA2000 (CAN)? Affects hardware
  (UART vs CAN transceiver) and firmware (STM32H723 has FDCAN, so both are
  feasible).
- Local storage medium: SD card over SPI vs onboard flash? Affects log
  capacity, wear, and whether logs are removable for offline analysis.
- True-wind accuracy: using GPS speed/course-over-ground rather than a
  heading sensor is a known simplification (wrong in current/leeway). Is
  that acceptable for v1, or does a heading source (compass) need to be
  in scope now?
- Any numeric targets: wind speed/direction accuracy, GPS fix rate,
  logging sample rate, minimum operating time on battery/boat power?
  None decided yet -- flagged rather than assumed in `requirements.sysml`.

## Impact If Ignored

Without resolving the NMEA variant and storage medium, `plan.md` can't
commit to concrete hardware (transceiver, storage part) or firmware
drivers -- architecture stays at the port-abstraction level only.

## Related Artifacts

- `requirements/model/domain.sysml`, `system.sysml`, `requirements.sysml`
  -- validated, `satisfy`-linked draft.
- `/mnt/c/git/sailing_meteo_station/ecosystem/*.sysml` -- prior project,
  source of the domain vocabulary/patterns reused here (wind vector
  algebra, part-def structure for a boat instrument unit).

## Recommended Next Step

Run `/ws.1-requirements` on this seed: turn the open questions above into
the Decisions block, and reconcile REQ-001..007 (already in
`requirements.sysml`) against the requirements.md structure.
