//! NMEA2000 wind output (REQ-006): encodes computed true wind as the
//! standard Wind Data PGN (130306) and transmits it on the boat's N2K
//! (CAN) bus.
//!
//! PGN 130306 byte layout is well-documented in the marine electronics
//! community (e.g. CANboat's PGN catalogue) but has never been checked
//! against a real NMEA2000 instrument here -- that's exactly what Task
//! 5's feasibility-gate AC (send a frame, see it land on a real bus) is
//! for. Byte-level correctness is [VERIFY IN WALKING SKELETON]-equivalent
//! (plan.md's terms): host tests below prove the encoding is internally
//! consistent, not that a real chartplotter parses it correctly.

use crate::wind_sensor::WindReading;

/// PGN 130306, "Wind Data".
pub const WIND_DATA_PGN: u32 = 0x1FD02;
/// Standard default priority for this PGN.
pub const WIND_DATA_PRIORITY: u8 = 2;
/// Standard NMEA2000 bus speed.
pub const NMEA2000_BITRATE: u32 = 250_000;

/// PGN 130306's "Wind Reference" field. Only `True` (ground-referenced,
/// i.e. relative to true North) is used here -- REQ-006 scopes this
/// output to computed *true* wind, not apparent.
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum WindReference {
    True = 0,
}

/// Builds the 29-bit extended CAN ID for a PGN 130306 broadcast from this
/// device (PDU2/broadcast format: priority | PGN | source address).
///
/// Mutation testing flags both `|` operators here as "missed" if
/// replaced with `^` -- that's a true equivalent mutant, not a test
/// gap: priority occupies bits 26-28, `WIND_DATA_PGN` bits 8-25, and
/// `source_address: u8` can only ever set bits 0-7, so the three fields
/// never share a set bit for any possible input, making `|` and `^`
/// identical for every input this function can ever receive.
pub fn can_id(source_address: u8) -> u32 {
    ((WIND_DATA_PRIORITY as u32) << 26) | (WIND_DATA_PGN << 8) | source_address as u32
}

/// Encodes one PGN 130306 data frame. `sid` is the sequence ID (any
/// incrementing counter shared across related PGNs; a fixed value is
/// fine when this is the only PGN sent).
///
/// `true_wind.valid == false` -> speed/angle fields are encoded as
/// "data not available" (0xFFFF, the standard NMEA2000 convention)
/// rather than either a stale number or no frame at all -- so a
/// compliant display shows "no data" instead of freezing on the last
/// good reading with no indication anything's wrong (REQ-006's negative
/// AC).
pub fn encode_wind_data(true_wind: WindReading, sid: u8) -> [u8; 8] {
    const NOT_AVAILABLE: u16 = 0xFFFF;

    let (speed_raw, angle_raw) = if true_wind.valid {
        let speed_raw = libm::roundf(true_wind.speed_mps / 0.01).clamp(0.0, 0xFFFEu16 as f32) as u16;
        let angle_rad = crate::angle::normalize(to_radians(true_wind.direction_deg), 2.0 * core::f32::consts::PI);
        let angle_raw = libm::roundf(angle_rad / 0.0001).clamp(0.0, 0xFFFEu16 as f32) as u16;
        (speed_raw, angle_raw)
    } else {
        (NOT_AVAILABLE, NOT_AVAILABLE)
    };

    let reference = WindReference::True as u8;

    // `reference & 0x0F | 0xF0` -- with `WindReference` currently having
    // only the `True = 0` variant, `reference` is always 0 here, which
    // makes `| 0xF0` and `^ 0xF0` equivalent (0 is the identity for XOR)
    // -- a true equivalent mutant, not a test gap, *for now*. It stops
    // being equivalent the day a second `WindReference` variant is added
    // and actually used, since XOR would then flip bits `|` wouldn't.
    [
        sid,
        (speed_raw & 0xFF) as u8,
        (speed_raw >> 8) as u8,
        (angle_raw & 0xFF) as u8,
        (angle_raw >> 8) as u8,
        reference & 0x0F | 0xF0, // low nibble = reference, high nibble reserved (all 1s)
        0xFF,                    // reserved
        0xFF,                    // reserved
    ]
}

fn to_radians(deg: f32) -> f32 {
    deg * (core::f32::consts::PI / 180.0)
}

#[cfg(feature = "embedded")]
mod hardware {
    use embassy_stm32::can::Frame;

    use super::{can_id, encode_wind_data};
    use crate::wind_sensor::WindReading;

    /// Transmits computed true wind as a PGN 130306 frame on the N2K bus.
    /// Owns no peripheral state itself (the caller owns the `Can` driver
    /// and calls `.write()`) -- this just builds the frame.
    pub fn wind_data_frame(true_wind: WindReading, source_address: u8, sid: u8) -> Frame {
        let data = encode_wind_data(true_wind, sid);
        Frame::new_extended(can_id(source_address), &data).expect("8-byte PGN 130306 frame is always valid")
    }
}

#[cfg(feature = "embedded")]
pub use hardware::wind_data_frame;

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_wind(speed: f32, direction: f32) -> WindReading {
        WindReading {
            speed_mps: speed,
            direction_deg: direction,
            valid: true,
        }
    }

    #[test]
    fn can_id_encodes_priority_pgn_and_source() {
        let id = can_id(0x23);
        assert_eq!(id, (2u32 << 26) | (0x1FD02 << 8) | 0x23);
    }

    #[test]
    fn encodes_speed_and_angle_at_pgn_resolution() {
        // 5.00 m/s -> 500 raw units (0.01 m/s per LSB), little-endian.
        // 90 degrees -> pi/2 rad -> 15708 raw units (0.0001 rad per LSB).
        let frame = encode_wind_data(valid_wind(5.0, 90.0), 7);
        assert_eq!(frame[0], 7); // sid
        let speed_raw = u16::from_le_bytes([frame[1], frame[2]]);
        assert_eq!(speed_raw, 500);
        let angle_raw = u16::from_le_bytes([frame[3], frame[4]]);
        assert!((angle_raw as i32 - 15708).abs() <= 1);
        // Full byte, not just the low nibble: the previous version of
        // this assertion (`frame[5] & 0x0F == 0`) couldn't tell a
        // correctly-built 0xF0 from a mangled 0x00, since both have a
        // zero low nibble.
        assert_eq!(frame[5], 0xF0);
    }

    #[test]
    fn invalid_true_wind_encodes_as_not_available() {
        let frame = encode_wind_data(WindReading::INVALID, 0);
        let speed_raw = u16::from_le_bytes([frame[1], frame[2]]);
        let angle_raw = u16::from_le_bytes([frame[3], frame[4]]);
        assert_eq!(speed_raw, 0xFFFF);
        assert_eq!(angle_raw, 0xFFFF);
    }

    #[test]
    fn direction_wraps_correctly_near_360() {
        let frame = encode_wind_data(valid_wind(1.0, 359.0), 0);
        let angle_raw = u16::from_le_bytes([frame[3], frame[4]]);
        // 359 degrees should be just under 2*pi radians, i.e. a large raw
        // value close to (but under) 2*pi/0.0001 ~= 62832, not negative
        // or wrapped to near-zero.
        assert!(angle_raw > 60000 && angle_raw < 62832);
    }
}
