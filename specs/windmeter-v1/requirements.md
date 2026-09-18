# Feature: windmeter-v1

## Decisions

<!-- Resolved 2026-09-18. -->

- [x] **NMEA bus variant.** Resolved: NMEA2000/CAN (not NMEA0183) -- STM32H723's FDCAN covers the transceiver interface.
- [x] **Power source.** Resolved: standalone battery, not tapped from the boat's 12V -- so the unit also works on dinghies with no onboard electrical system.
- [x] **Local display.** Resolved: none in v1. Idea for later: a separate wireless-connected display unit -- parked in `specs/_ideas.md`, not designed now.
- [x] **True-wind accuracy for v1.** Resolved: accept GPS speed/course-over-ground as boat velocity (documented limitation under current/leeway); heading sensor deferred.
- [x] **Log retrieval.** Resolved: removable SD card.

---

## Context

### User story

As the boat's owner/skipper, I want apparent and true wind sensed, computed, logged, and shared on my instrument bus, so I can see wind data on my existing instruments while sailing and review it afterward.

### Acceptance criteria

- Given the wind sensor is connected and powered, when running, then apparent wind speed/direction relative to the boat's centerline is reported at a regular interval. (REQ-001)
- Given the wind sensor is disconnected or unresponsive, then the apparent wind reading is flagged invalid, not stale/fabricated. (REQ-001)
- Given a valid apparent wind reading and a valid GPS fix, then true wind speed/direction is reported, derived from apparent wind and GPS speed/course-over-ground. (REQ-002)
- Given the GPS fix is invalid or stale, then true wind is flagged invalid rather than computed from a bad fix. (REQ-002)
- Given the onboard GPS has a fix, then position, speed- and course-over-ground are available to the true-wind computation independent of the boat's existing instrument bus. (REQ-003)
- Given the device is running, then apparent wind, true wind, and the underlying GPS fix are appended to local persistent storage over time. (REQ-005)
- Given local storage is full or fails, then the device keeps sensing/broadcasting and surfaces the fault rather than silently dropping data. (REQ-005)
- Given computed apparent/true wind, when connected to the boat's NMEA bus, then that data is broadcast at a regular interval. (REQ-006)
- Given no valid true wind is currently available, then the device omits or marks that sentence invalid rather than sending a stale reading. (REQ-006)
- Given the `WindSensor` abstraction (`system.sysml`), when the concrete sensor is swapped (cup-and-vane -> ultrasonic), then processing/logging/NMEA-output need no redesign -- verified at architecture review, not runtime. (REQ-004)
- Given the unit is installed on a boat or dinghy with no onboard 12V supply, then it still operates fully, powered by its own battery. (REQ-008)

### Out of scope

- Ultrasonic wind sensor hardware (architecture supports it; not built this cycle).
- Multi-boat/regatta features (fleet tracking, race committee, web viewer).
- A separate wireless display unit (parked as an idea, see `specs/_ideas.md`).
- Numeric targets for accuracy, GPS fix rate, sample rate, minimum operating time -- deferred to after real-world testing, not guessed now.
- Battery chemistry/capacity, charging method, and minimum runtime between charges -- plan-level detail (REQ-008 open items).

### Open questions

- [ASSUMPTION: NMEA2000 PGNs to transmit, and log file format, are plan-level detail now that the bus variant is decided.]

### Interface contract

- Wind sensor port: speed + direction (`WindSensorPort`, `system.sysml`).
- GPS port: position, speed-/course-over-ground (`GpsPort`).
- NMEA2000 PGNs and log record format: plan-level detail, specified in `plan.md`.

### Related

`requirements/model/{domain,system,requirements}.sysml` (formal record, `satisfy`-linked to `windMeterV1`) · `specs/windmeter-v1/seed.md`.
