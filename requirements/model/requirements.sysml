package WindMeterRequirements {
    doc /*
     * Requirements for the windmeter device, satisfied against the
     * WindMeterSystem architecture. Requirements whose thresholds have not
     * been decided yet are marked as open items in their doc rather than
     * given invented numbers.
     */

    private import WindMeterSystem::*;

    requirement def SenseApparentWindReqDef {
        doc /* The system shall measure apparent wind speed and direction
             * relative to the boat's centerline. */
        subject unit: WindMeterUnit;
    }
    requirement <'REQ-001'> senseApparentWind : SenseApparentWindReqDef;
    satisfy senseApparentWind by WindMeterSystem::windMeterV1;

    requirement def ComputeTrueWindReqDef {
        doc /* The system shall compute true wind (speed and direction)
             * from apparent wind and the boat's GPS-derived
             * velocity-over-ground (speed-over-ground + course-over-ground).
             *
             * Open item: using velocity-over-ground rather than a heading
             * sensor / paddlewheel-log-derived velocity-through-water is a
             * deliberate first-iteration simplification. It is accurate
             * only when there is no current and no leeway; revisit once a
             * heading source is added. */
        subject unit: WindMeterUnit;
    }
    requirement <'REQ-002'> computeTrueWind : ComputeTrueWindReqDef;
    satisfy computeTrueWind by WindMeterSystem::windMeterV1;

    requirement def OnboardGpsReqDef {
        doc /* The system shall obtain position, speed-over-ground and
             * course-over-ground from an onboard GPS receiver module (not
             * from the boat's existing instrument bus). */
        subject unit: WindMeterUnit;
    }
    requirement <'REQ-003'> onboardGps : OnboardGpsReqDef;
    satisfy onboardGps by WindMeterSystem::windMeterV1;

    requirement def SwappableWindSensorReqDef {
        doc /* The architecture shall let the wind-sensing element be
             * replaced (cup-and-vane -> ultrasonic) without redesign of
             * the processing, logging or NMEA-output subsystems. Initial
             * hardware release uses a cup-and-vane sensor; an ultrasonic
             * sensor is a planned follow-on, not a v1 deliverable. */
        subject unit: WindMeterUnit;
    }
    requirement <'REQ-004'> swappableWindSensor : SwappableWindSensorReqDef;
    satisfy swappableWindSensor by WindMeterSystem::windMeterV1;

    requirement def LocalLoggingReqDef {
        doc /* The system shall log computed apparent and true wind data,
             * together with the underlying GPS fix, to a removable SD
             * card for later analysis (decided: SD card over SPI, so the
             * card can be pulled and read on a laptop).
             *
             * Open items: log format, retention/rotation policy, and
             * sample rate are not yet decided. */
        subject unit: WindMeterUnit;
    }
    requirement <'REQ-005'> localLogging : LocalLoggingReqDef;
    satisfy localLogging by WindMeterSystem::windMeterV1;

    requirement def NmeaOutputReqDef {
        doc /* The system shall output computed apparent and true wind data
             * onto the boat's NMEA2000 (CAN) instrument bus so it can be
             * consumed by a chartplotter or other NMEA2000-capable
             * instruments (decided: NMEA2000 over NMEA0183; STM32H723's
             * FDCAN peripheral covers the transceiver interface).
             *
             * Open item: which PGNs to transmit, and at what rate, is not
             * yet decided. */
        subject unit: WindMeterUnit;
    }
    requirement <'REQ-006'> nmeaOutputReq : NmeaOutputReqDef;
    satisfy nmeaOutputReq by WindMeterSystem::windMeterV1;

    requirement def BatteryPoweredReqDef {
        doc /* The system shall be powered by its own self-contained
             * battery rather than depend on the host boat's 12V
             * electrical system, so it can be installed on boats/dinghies
             * that have no onboard power supply at all.
             *
             * Open items: battery chemistry/capacity, charging method
             * (e.g. USB vs solar), and minimum operating time between
             * charges are not yet decided. */
        subject unit: WindMeterUnit;
    }
    requirement <'REQ-008'> batteryPowered : BatteryPoweredReqDef;
    satisfy batteryPowered by WindMeterSystem::windMeterV1;

    requirement def EmbeddedPlatformReqDef {
        doc /* The firmware shall run on an STM32 microcontroller, written
             * in Rust. Target chip may change; the current development
             * target is STM32H723ZG (see Cargo.toml / memory.x). */
        subject unit: WindMeterUnit;
    }
    requirement <'REQ-007'> embeddedPlatform : EmbeddedPlatformReqDef;
    satisfy embeddedPlatform by WindMeterSystem::windMeterV1;
}
