package WindMeterSystem {
    doc /*
     * Architecture of the windmeter device itself: one boat-mounted unit
     * that senses apparent wind, reads GPS, computes true wind, logs
     * locally, and outputs onto the boat's NMEA bus.
     *
     * The wind-sensing element is modeled as an abstract WindSensor so the
     * rest of the system (processing/logging/output) does not need to
     * change when the cup-and-vane sensor is later swapped for an
     * ultrasonic one - see REQ-SwappableWindSensor.
     */

    private import WindMeterDomain::*;

    port def WindSensorPort {
        out item reading: Wind;
    }

    port def GpsPort {
        out attribute position: GeoPosition;
        out attribute speedOverGround: ISQ::SpeedValue;
        out attribute courseOverGround: ISQ::AngularMeasureValue;
    }

    port def NmeaPort {
        doc /* NMEA2000 (CAN) link onto the boat's instrument bus, via the
             * STM32H723's FDCAN peripheral. Which PGNs and at what rate is
             * still open - see REQ-006 (nmeaOutputReq) in the requirements
             * package. */
    }

    part def WindSensor {
        doc /* Abstract wind-sensing element: produces apparent wind speed
             * and direction relative to the boat's centerline. */
        port reading: WindSensorPort;
    }

    part def CupAndVaneWindSensor :> WindSensor {
        doc /* Initial hardware: pulse (reed/Hall) anemometer for speed,
             * a continuous potentiometer wind vane for direction.
             *
             * Selected: Davis Instruments 6410 (Vantage Pro2 anemometer +
             * vane) - stainless steel bearings, 200 mph wind-tunnel
             * tested, field-proven 10+ year lifespans (incl. DIY marine/
             * boat use). Chosen over the initially-picked Argent Data
             * Systems / SparkFun Weather Meter Kit (SEN-15901) for
             * ruggedness - see hardware-sourcing.md for the comparison
             * against SEN-15901 and other boat-specific options (Garmin
             * GWS 10, B&G 508) considered and not chosen. Anemometer
             * constant 2.25 mph per pulse/second; vane is a linear ~20k
             * ohm pot, not a resistor ladder. See src/wind_sensor.rs for
             * the calibration. */
    }

    part def UltrasonicWindSensor :> WindSensor {
        doc /* Planned future replacement: solid-state ultrasonic
             * transducer array, no moving parts. */
    }

    part def GpsReceiver {
        doc /* Onboard GPS module (UART, NMEA0183) providing position,
             * speed-over-ground and course-over-ground.
             *
             * Selected: u-blox NEO-6M breakout (e.g. GY-NEO6MV2) - cheap,
             * widely available, 9600 baud NMEA0183 default. NEO-M8N is a
             * pin-compatible upgrade (better accuracy/multi-GNSS) if
             * NEO-6M's accuracy proves insufficient. */
        port fix: GpsPort;
    }

    part def DataLogger {
        doc /* Local persistent storage for logged wind/GPS records: a
             * removable SD card over SPI, so the card can be pulled and
             * read on a laptop. Log format and retention policy are still
             * open - see REQ-005 (localLogging).
             *
             * Selected: generic microSD SPI breakout module (3.3V logic,
             * no onboard regulator needed - matches the STM32's 3.3V I/O). */
    }

    part def PowerSupply {
        doc /* Self-contained battery power - the unit does not depend on
             * the host boat's 12V system, so it can be installed on
             * boats/dinghies with no onboard electrical supply. Battery
             * chemistry/capacity and charging method are still open - see
             * REQ-008 (batteryPowered).
             *
             * Selected for prototyping: single-cell Li-ion/LiPo (e.g.
             * 18650) + TP4056 USB-C charge/protection module. A
             * buck/boost regulator to the board's supply rail(s) is not
             * yet selected. */
    }

    part def NmeaOutput {
        doc /* Broadcasts computed apparent/true wind onto the boat's
             * instrument bus.
             *
             * Selected: Waveshare SN65HVD230 CAN transceiver breakout
             * (3.3V logic, matches the STM32 directly - no level shifter
             * needed) between the MCU's FDCAN TX/RX and the physical
             * NMEA2000 bus. */
        port bus: NmeaPort;
    }

    part def ProcessingUnit {
        doc /* STM32 microcontroller running the Rust/Embassy firmware:
             * reads the wind sensor and GPS, computes true wind, and
             * drives the logger and NMEA output.
             *
             * Selected devkit: NUCLEO-H723ZG (ST's official STM32H723ZG
             * board, onboard ST-LINK/V3 debug probe, Arduino Uno V3 +
             * ST Zio/Morpho headers). */
        port windIn: WindSensorPort;
        port gpsIn: GpsPort;
        port nmeaOut: NmeaPort;
    }

    part def WindMeterUnit {
        part windSensor: WindSensor;
        part gps: GpsReceiver;
        part mcu: ProcessingUnit;
        part logger: DataLogger;
        part nmeaOut: NmeaOutput;
        part power: PowerSupply;

        connect windSensor.reading to mcu.windIn;
        connect gps.fix to mcu.gpsIn;
        connect mcu.nmeaOut to nmeaOut.bus;
    }

    // First hardware build: cup-and-vane sensor.
    part windMeterV1: WindMeterUnit {
        part :>> windSensor: CupAndVaneWindSensor;
    }
}
