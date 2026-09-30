# windmeter: Project Description

windmeter is a boat-mounted instrument that senses apparent wind, reads GPS, computes true wind, logs both locally, and broadcasts the result on the boat's NMEA2000 instrument bus. This document is generated from requirements/model/\*.sysml via OpenSysML's document queries (sysml -render-document) -- every table below reads the live model, not a hand-maintained copy of it.

True wind is derived from apparent wind and the boat's own GPS velocity over ground (the classic wind triangle: trueWind = apparentWind - velocityOverGround), rather than a heading sensor or paddlewheel log -- accurate only absent current and leeway, a deliberate first-iteration simplification (see WindMeterDomain::Boat and REQ-002's documented open item below).

## Requirements

*Requirement index*

| shortName | name |
| --- | --- |
| REQ-001 | senseApparentWind |
| REQ-002 | computeTrueWind |
| REQ-003 | onboardGps |
| REQ-004 | swappableWindSensor |
| REQ-005 | localLogging |
| REQ-006 | nmeaOutputReq |
| REQ-007 | embeddedPlatform |
| REQ-008 | batteryPowered |

**SenseApparentWindReqDef** — The system shall measure apparent wind speed and direction relative to the boat's centerline.

**ComputeTrueWindReqDef** — The system shall compute true wind (speed and direction) from apparent wind and the boat's GPS-derived velocity-over-ground (speed-over-ground + course-over-ground).  Open item: using velocity-over-ground rather than a heading sensor / paddlewheel-log-derived velocity-through-water is a deliberate first-iteration simplification. It is accurate only when there is no current and no leeway; revisit once a heading source is added.

**OnboardGpsReqDef** — The system shall obtain position, speed-over-ground and course-over-ground from an onboard GPS receiver module (not from the boat's existing instrument bus).

**SwappableWindSensorReqDef** — The architecture shall let the wind-sensing element be replaced (cup-and-vane -> ultrasonic) without redesign of the processing, logging or NMEA-output subsystems. Initial hardware release uses a cup-and-vane sensor; an ultrasonic sensor is a planned follow-on, not a v1 deliverable.

**LocalLoggingReqDef** — The system shall log computed apparent and true wind data, together with the underlying GPS fix, to a removable SD card for later analysis (decided: SD card over SPI, so the card can be pulled and read on a laptop).  Open items: log format, retention/rotation policy, and sample rate are not yet decided.

**NmeaOutputReqDef** — The system shall output computed apparent and true wind data onto the boat's NMEA2000 (CAN) instrument bus so it can be consumed by a chartplotter or other NMEA2000-capable instruments (decided: NMEA2000 over NMEA0183; STM32H723's FDCAN peripheral covers the transceiver interface).  Open item: which PGNs to transmit, and at what rate, is not yet decided.

**BatteryPoweredReqDef** — The system shall be powered by its own self-contained battery rather than depend on the host boat's 12V electrical system, so it can be installed on boats/dinghies that have no onboard power supply at all.  Open items: battery chemistry/capacity, charging method (e.g. USB vs solar), and minimum operating time between charges are not yet decided.

**EmbeddedPlatformReqDef** — The firmware shall run on an STM32 microcontroller, written in Rust. Target chip may change; the current development target is STM32H723ZG (see Cargo.toml / memory.x).

## System Architecture

WindMeterUnit's six parts, each swappable for another implementation of the same role (e.g. REQ-004's ultrasonic-sensor swap-in) without touching the rest:

**WindSensor** — Abstract wind-sensing element: produces apparent wind speed and direction relative to the boat's centerline.

**GpsReceiver** — Onboard GPS module (UART, NMEA0183) providing position, speed-over-ground and course-over-ground.  Selected: u-blox NEO-6M breakout (e.g. GY-NEO6MV2) - cheap, widely available, 9600 baud NMEA0183 default. NEO-M8N is a pin-compatible upgrade (better accuracy/multi-GNSS) if NEO-6M's accuracy proves insufficient.

**ProcessingUnit** — STM32 microcontroller running the Rust/Embassy firmware: reads the wind sensor and GPS, computes true wind, and drives the logger and NMEA output.  Selected devkit: NUCLEO-H723ZG (ST's official STM32H723ZG board, onboard ST-LINK/V3 debug probe, Arduino Uno V3 + ST Zio/Morpho headers).

**DataLogger** — Local persistent storage for logged wind/GPS records: a removable SD card over SPI, so the card can be pulled and read on a laptop. Log format and retention policy are still open - see REQ-005 (localLogging).  Selected: generic microSD SPI breakout module (3.3V logic, no onboard regulator needed - matches the STM32's 3.3V I/O).

**NmeaOutput** — Broadcasts computed apparent/true wind onto the boat's instrument bus.  Selected: Waveshare SN65HVD230 CAN transceiver breakout (3.3V logic, matches the STM32 directly - no level shifter needed) between the MCU's FDCAN TX/RX and the physical NMEA2000 bus.

**PowerSupply** — Self-contained battery power - the unit does not depend on the host boat's 12V system, so it can be installed on boats/dinghies with no onboard electrical supply. Battery chemistry/capacity and charging method are still open - see REQ-008 (batteryPowered).  Selected for prototyping: single-cell Li-ion/LiPo (e.g. 18650) + TP4056 USB-C charge/protection module. A buck/boost regulator to the board's supply rail(s) is not yet selected.

The first hardware build, windMeterV1, uses CupAndVaneWindSensor (a Davis Instruments 6410 anemometer/vane) as its concrete windSensor -- see hardware-sourcing.md for the ruggedness comparison against the sensors not chosen.

## Traceability

*Every requirement and what satisfies it*

| shortName | name | satisfiedBy |
| --- | --- | --- |
| REQ-001 | senseApparentWind | windMeterV1 |
| REQ-002 | computeTrueWind | windMeterV1 |
| REQ-003 | onboardGps | windMeterV1 |
| REQ-004 | swappableWindSensor | windMeterV1 |
| REQ-005 | localLogging | windMeterV1 |
| REQ-006 | nmeaOutputReq | windMeterV1 |
| REQ-007 | embeddedPlatform | windMeterV1 |
| REQ-008 | batteryPowered | windMeterV1 |
