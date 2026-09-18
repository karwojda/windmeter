//! Local logging (REQ-005): one CSV record per sample tick, covering
//! apparent wind, true wind and the underlying GPS fix, so a session can
//! be reviewed after the fact (REQ-005) even on a boat with no onboard
//! power (REQ-008, verified by running this on battery alone).

use core::fmt::Write;

use crate::gps::GpsFix;
use crate::wind_sensor::WindReading;

/// One sample tick's worth of data to log.
#[derive(Debug, Clone, Copy)]
pub struct LogRecord {
    pub timestamp_ms: u64,
    pub apparent: WindReading,
    pub true_wind: WindReading,
    pub gps: GpsFix,
}

/// A fixed-size, no_std/no-alloc buffer that `write!` can target.
struct BufWriter<'a> {
    buf: &'a mut [u8],
    len: usize,
}

impl<'a> Write for BufWriter<'a> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let bytes = s.as_bytes();
        if self.len + bytes.len() > self.buf.len() {
            return Err(core::fmt::Error);
        }
        self.buf[self.len..self.len + bytes.len()].copy_from_slice(bytes);
        self.len += bytes.len();
        Ok(())
    }
}

impl LogRecord {
    /// CSV header, without a trailing newline.
    pub const CSV_HEADER: &'static str =
        "timestamp_ms,apparent_valid,apparent_speed_mps,apparent_dir_deg,true_valid,true_speed_mps,true_dir_deg,gps_valid,latitude_deg,longitude_deg,sog_mps,cog_deg";

    /// Formats this record as one CSV line (no trailing newline) into
    /// `buf`, returning the written slice, or `None` if it doesn't fit.
    pub fn format_csv<'a>(&self, buf: &'a mut [u8]) -> Option<&'a str> {
        let mut w = BufWriter { buf, len: 0 };
        write!(
            w,
            "{},{},{},{},{},{},{},{},{},{},{},{}",
            self.timestamp_ms,
            self.apparent.valid,
            self.apparent.speed_mps,
            self.apparent.direction_deg,
            self.true_wind.valid,
            self.true_wind.speed_mps,
            self.true_wind.direction_deg,
            self.gps.valid,
            self.gps.latitude_deg,
            self.gps.longitude_deg,
            self.gps.sog_mps,
            self.gps.cog_deg,
        )
        .ok()?;
        let len = w.len;
        core::str::from_utf8(&buf[..len]).ok()
    }
}

#[cfg(feature = "embedded")]
mod hardware {
    use embassy_stm32::gpio::Output;
    use embassy_stm32::spi::Spi;
    use embassy_time::Delay;
    use embedded_hal_bus::spi::ExclusiveDevice;
    use embedded_sdmmc::{Mode, SdCard, TimeSource, Timestamp, VolumeIdx, VolumeManager};

    use super::LogRecord;

    /// `embedded-sdmmc` needs a `TimeSource` for directory-entry
    /// timestamps. No RTC is wired up yet, so this always reports a fixed
    /// epoch -- harmless (files still write/read correctly), just not
    /// meaningful file timestamps. Revisit if that turns out to matter.
    struct FixedTimeSource;
    impl TimeSource for FixedTimeSource {
        fn get_timestamp(&self) -> Timestamp {
            Timestamp {
                year_since_1970: 55, // 2025
                zero_indexed_month: 0,
                zero_indexed_day: 0,
                hours: 0,
                minutes: 0,
                seconds: 0,
            }
        }
    }

    type SpiDevice<'d> =
        ExclusiveDevice<Spi<'d, embassy_stm32::mode::Blocking, embassy_stm32::spi::mode::Master>, Output<'d>, Delay>;

    /// Appends log records to `WINDLOG.CSV` on a FAT-formatted SD card
    /// over SPI. Writes the CSV header once, the first time the file is
    /// created.
    pub struct DataLogger<'d> {
        volume_mgr: VolumeManager<SdCard<SpiDevice<'d>, Delay>, FixedTimeSource>,
    }

    impl<'d> DataLogger<'d> {
        pub fn new(spi: Spi<'d, embassy_stm32::mode::Blocking, embassy_stm32::spi::mode::Master>, cs: Output<'d>) -> Self {
            let spi_dev = ExclusiveDevice::new(spi, cs, Delay).expect("SD card SPI device");
            let sdcard = SdCard::new(spi_dev, Delay);
            Self {
                volume_mgr: VolumeManager::new(sdcard, FixedTimeSource),
            }
        }

        /// Appends one record as a CSV line. Returns `Err` on any storage
        /// fault (full card, I/O error, ...) -- callers must not treat
        /// that as fatal (REQ-005's negative AC): keep sensing/computing
        /// and surface the fault instead of crashing.
        pub fn append(&mut self, record: &LogRecord) -> Result<(), &'static str> {
            let mut buf = [0u8; 160];
            let line = record.format_csv(&mut buf).ok_or("record too long for buffer")?;

            let volume = self
                .volume_mgr
                .open_volume(VolumeIdx(0))
                .map_err(|_| "open_volume failed")?;
            let root_dir = volume.open_root_dir().map_err(|_| "open_root_dir failed")?;
            let existed = root_dir.find_directory_entry("WINDLOG.CSV").is_ok();
            let file = root_dir
                .open_file_in_dir("WINDLOG.CSV", Mode::ReadWriteCreateOrAppend)
                .map_err(|_| "open_file_in_dir failed")?;

            if !existed {
                file.write(LogRecord::CSV_HEADER.as_bytes()).map_err(|_| "write header failed")?;
                file.write(b"\n").map_err(|_| "write header newline failed")?;
            }
            file.write(line.as_bytes()).map_err(|_| "write record failed")?;
            file.write(b"\n").map_err(|_| "write newline failed")?;
            file.flush().map_err(|_| "flush failed")?;
            Ok(())
        }
    }
}

#[cfg(feature = "embedded")]
pub use hardware::DataLogger;

#[cfg(test)]
mod tests {
    use super::*;

    fn wind(speed: f32, direction: f32, valid: bool) -> WindReading {
        WindReading {
            speed_mps: speed,
            direction_deg: direction,
            valid,
        }
    }

    fn fix(sog: f32, cog: f32) -> GpsFix {
        GpsFix {
            latitude_deg: 42.5,
            longitude_deg: -71.25,
            sog_mps: sog,
            cog_deg: cog,
            valid: true,
        }
    }

    #[test]
    fn formats_a_full_record_as_csv() {
        let record = LogRecord {
            timestamp_ms: 12_345,
            apparent: wind(5.0, 90.0, true),
            true_wind: wind(3.0, 45.0, true),
            gps: fix(2.0, 180.0),
        };
        let mut buf = [0u8; 160];
        let line = record.format_csv(&mut buf).expect("fits");
        assert_eq!(
            line,
            "12345,true,5,90,true,3,45,true,42.5,-71.25,2,180"
        );
    }

    #[test]
    fn invalid_readings_are_still_logged_with_valid_false() {
        let record = LogRecord {
            timestamp_ms: 1,
            apparent: WindReading::INVALID,
            true_wind: WindReading::INVALID,
            gps: GpsFix::INVALID,
        };
        let mut buf = [0u8; 160];
        let line = record.format_csv(&mut buf).expect("fits");
        assert!(line.starts_with("1,false,0,0,false,0,0,false,0,0,0,0"));
    }

    #[test]
    fn returns_none_when_buffer_too_small() {
        let record = LogRecord {
            timestamp_ms: 1,
            apparent: WindReading::INVALID,
            true_wind: WindReading::INVALID,
            gps: GpsFix::INVALID,
        };
        let mut buf = [0u8; 4];
        assert!(record.format_csv(&mut buf).is_none());
    }

    #[test]
    fn header_names_every_column_in_the_record() {
        // One header field per comma-separated value the record produces.
        let header_fields = LogRecord::CSV_HEADER.split(',').count();
        let record = LogRecord {
            timestamp_ms: 1,
            apparent: WindReading::INVALID,
            true_wind: WindReading::INVALID,
            gps: GpsFix::INVALID,
        };
        let mut buf = [0u8; 160];
        let line = record.format_csv(&mut buf).unwrap();
        assert_eq!(header_fields, line.split(',').count());
    }
}
