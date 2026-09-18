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
             * potentiometer or encoder for direction. */
    }

    part def UltrasonicWindSensor :> WindSensor {
        doc /* Planned future replacement: solid-state ultrasonic
             * transducer array, no moving parts. */
    }

    part def GpsReceiver {
        doc /* Onboard GPS module (UART, NMEA0183) providing position,
             * speed-over-ground and course-over-ground. */
        port fix: GpsPort;
    }

    part def DataLogger {
        doc /* Local persistent storage for logged wind/GPS records: a
             * removable SD card over SPI, so the card can be pulled and
             * read on a laptop. Log format and retention policy are still
             * open - see REQ-005 (localLogging). */
    }

    part def PowerSupply {
        doc /* Self-contained battery power - the unit does not depend on
             * the host boat's 12V system, so it can be installed on
             * boats/dinghies with no onboard electrical supply. Battery
             * chemistry/capacity and charging method are still open - see
             * REQ-008 (batteryPowered). */
    }

    part def NmeaOutput {
        doc /* Broadcasts computed apparent/true wind onto the boat's
             * instrument bus. */
        port bus: NmeaPort;
    }

    part def ProcessingUnit {
        doc /* STM32 microcontroller running the Rust/Embassy firmware:
             * reads the wind sensor and GPS, computes true wind, and
             * drives the logger and NMEA output. */
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
