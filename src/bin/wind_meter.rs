#![cfg(feature = "embedded")]
#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_stm32::adc::Adc;
use embassy_stm32::exti::ExtiInput;
use embassy_stm32::gpio::{Level, Output, OutputType, Pull, Speed};
use embassy_stm32::time::khz;
use embassy_stm32::timer::simple_pwm::{*,PwmPin};
use embassy_stm32::interrupt::typelevel::EXTI0 as ExtiLine0;
use embassy_stm32::can::CanConfigurator;
use embassy_stm32::spi::Spi;
use embassy_stm32::usart::UartRx;
use embassy_stm32::{bind_interrupts, can, exti, peripherals, spi, usart, Config, Peri};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::watch::Watch;
use embassy_time::{Instant, Timer};
use embassy_stm32::peripherals::{PE1, PB14, TIM12};
use {defmt_rtt as _, panic_probe as _};
use wind_meter::gps::{GpsFix, GpsReceiver};
use wind_meter::logger::{DataLogger, LogRecord};
use wind_meter::nmea2000::{self, wind_data_frame};
use wind_meter::wind_compute::compute_true_wind;
use wind_meter::wind_sensor::{CupAndVaneWindSensor, WindCalibration, WindReading};
use wind_meter::windmeter::Wind;

// TODO: this device's NMEA2000 source address -- any value 0..252 that
// doesn't collide with another device already on the bus. Real N2K
// networks run address claim/negotiation; a fixed value is a placeholder
// until that's implemented, confirmed during the hardware verification
// pass (tasks.md Task 5).
const NMEA2000_SOURCE_ADDRESS: u8 = 0x42;

// Latest-value channels (REQ-002): wind_sensor_task and gps_task each
// publish their newest reading here; wind_compute_task reads whatever is
// currently published from both, on its own tick, rather than waiting on
// either producer individually -- the two sample at different natural
// rates (sensor pulses vs GPS fixes) and true wind should use the
// freshest available pair, not resynchronize to the slower one.
// N=2: each has two readers (wind_compute_task and logger_task).
static APPARENT_WIND: Watch<CriticalSectionRawMutex, WindReading, 2> = Watch::new();
static GPS_FIX: Watch<CriticalSectionRawMutex, GpsFix, 2> = Watch::new();
// N=2: logger_task and nmea2000_task each read the computed true wind.
static TRUE_WIND: Watch<CriticalSectionRawMutex, WindReading, 2> = Watch::new();

// TODO: placeholder pin/peripheral assignment throughout this file --
// confirm against the real wiring during the hardware verification pass
// (tasks.md) and update here. Nothing downstream of the driver
// constructors is affected by which pins/peripherals these turn out to be.
bind_interrupts!(struct Irqs {
    EXTI0 => exti::InterruptHandler<ExtiLine0>;
    USART3 => usart::InterruptHandler<peripherals::USART3>;
    DMA1_STREAM0 => embassy_stm32::dma::InterruptHandler<peripherals::DMA1_CH0>;
    FDCAN1_IT0 => can::IT0InterruptHandler<peripherals::FDCAN1>;
    FDCAN1_IT1 => can::IT1InterruptHandler<peripherals::FDCAN1>;
});

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let config = Config::default();

    let p = embassy_stm32::init(config);
    info!("Hello World!");

    let _ = spawner.spawn(double_blinky_manually_assigned(spawner, p.PE1));

    let _ = spawner.spawn(blink_red_led(spawner, p.PB14, p.TIM12));

    let pulse_pin = ExtiInput::new(p.PA0, p.EXTI0, Pull::Up, Irqs);
    let direction_adc = Adc::new(p.ADC1);
    let _ = spawner.spawn(wind_sensor_task(pulse_pin, direction_adc, p.PA1));

    // u-blox NEO-6M/NEO-M8N (and most hobby GPS modules) output NMEA0183
    // at 9600 baud by default -- embassy-stm32's usart::Config default is
    // 115200, which would silently desync the parser. Confirmed via the
    // NEO-6M/NEO-M8N datasheets during hardware research.
    let mut gps_uart_config = usart::Config::default();
    gps_uart_config.baudrate = 9600;
    let gps_rx = UartRx::new(p.USART3, p.PD9, p.DMA1_CH0, Irqs, gps_uart_config).expect("GPS UART config");
    let _ = spawner.spawn(gps_task(gps_rx));

    let _ = spawner.spawn(wind_compute_task());

    // SCK/MISO on PA5/PA6, MOSI on PB5 -- this is the Nucleo-H723ZG's
    // actual Arduino/Zio-header SPI1 routing (confirmed against ST's
    // pinout docs; PA7 -- a chip-level-valid but board-unrouted MOSI
    // option -- compiled fine earlier but isn't what's on the header).
    let mut spi_config = spi::Config::default();
    spi_config.frequency = embassy_stm32::time::mhz(1);
    let spi = Spi::new_blocking(p.SPI1, p.PA5, p.PB5, p.PA6, spi_config);
    let cs = Output::new(p.PA4, Level::High, Speed::Low);
    let data_logger = DataLogger::new(spi, cs);
    let _ = spawner.spawn(logger_task(data_logger));

    let mut can_config = CanConfigurator::new(p.FDCAN1, p.PD0, p.PD1, Irqs);
    can_config.set_bitrate(nmea2000::NMEA2000_BITRATE);
    let can = can_config.into_normal_mode();
    let _ = spawner.spawn(nmea2000_task(can));
}

/// REQ-006: transmits computed true wind as an NMEA2000 (PGN 130306) frame
/// once per sample period. No valid true wind -> the frame still goes out
/// but with "data not available" fields, never a stale number silently
/// re-sent (see `nmea2000::encode_wind_data`'s doc comment).
#[embassy_executor::task]
async fn nmea2000_task(mut can: can::Can<'static>) {
    const SAMPLE_PERIOD_S: u64 = 1;

    let mut true_wind_rx = TRUE_WIND.receiver().expect("nmea2000_task receiver");
    let mut sid: u8 = 0;

    loop {
        Timer::after_secs(SAMPLE_PERIOD_S).await;

        let true_wind = true_wind_rx.try_get().unwrap_or(WindReading::INVALID);
        let frame = wind_data_frame(true_wind, NMEA2000_SOURCE_ADDRESS, sid);
        sid = sid.wrapping_add(1);
        can.write(&frame).await;

        if true_wind.valid {
            info!("nmea2000: sent wind data ({} m/s @ {} deg)", true_wind.speed_mps, true_wind.direction_deg);
        } else {
            info!("nmea2000: sent wind data (not available)");
        }
    }
}

/// REQ-005 + REQ-008: appends one CSV record per sample period, covering
/// apparent wind, true wind and the GPS fix, to the SD card. A storage
/// fault is surfaced (RTT) but never stops the loop -- sensing/broadcasting
/// keeps running regardless (REQ-005's negative AC).
#[embassy_executor::task]
async fn logger_task(mut data_logger: DataLogger<'static>) {
    const SAMPLE_PERIOD_S: u64 = 1;

    let mut apparent_rx = APPARENT_WIND.receiver().expect("logger_task receiver");
    let mut true_wind_rx = TRUE_WIND.receiver().expect("logger_task receiver");
    let mut gps_rx = GPS_FIX.receiver().expect("logger_task receiver");

    loop {
        Timer::after_secs(SAMPLE_PERIOD_S).await;

        let record = LogRecord {
            timestamp_ms: Instant::now().as_millis(),
            apparent: apparent_rx.try_get().unwrap_or(WindReading::INVALID),
            true_wind: true_wind_rx.try_get().unwrap_or(WindReading::INVALID),
            gps: gps_rx.try_get().unwrap_or(GpsFix::INVALID),
        };

        if let Err(e) = data_logger.append(&record) {
            warn!("log write failed (storage fault?): {}", e);
        }
    }
}

/// REQ-002: combines the latest apparent wind and GPS fix into true wind
/// once per sample period, and prints it -- or that it's currently
/// invalid -- over RTT.
#[embassy_executor::task]
async fn wind_compute_task() {
    const SAMPLE_PERIOD_S: u64 = 1;

    let mut apparent_rx = APPARENT_WIND.receiver().expect("single wind_compute_task receiver");
    let mut gps_rx = GPS_FIX.receiver().expect("single wind_compute_task receiver");

    loop {
        Timer::after_secs(SAMPLE_PERIOD_S).await;

        let apparent = apparent_rx.try_get().unwrap_or(WindReading::INVALID);
        let gps = gps_rx.try_get().unwrap_or(GpsFix::INVALID);
        let true_wind = compute_true_wind(apparent, gps);
        TRUE_WIND.sender().send(true_wind);

        if true_wind.valid {
            info!(
                "true wind: {} m/s @ {} deg",
                true_wind.speed_mps, true_wind.direction_deg
            );
        } else {
            info!("true wind: invalid (apparent wind and/or GPS fix unavailable)");
        }
    }
}

/// REQ-003: reads the onboard GPS's NMEA0183 UART stream once per sample
/// period and prints the live fix -- or that it's currently invalid --
/// over RTT.
#[embassy_executor::task]
async fn gps_task(rx: UartRx<'static, embassy_stm32::mode::Async>) {
    const MAX_FIX_AGE_S: f32 = 5.0;
    let mut gps = GpsReceiver::new(rx, MAX_FIX_AGE_S);

    loop {
        let fix = gps.sample().await;
        GPS_FIX.sender().send(fix);
        if fix.valid {
            info!(
                "gps fix: {} , {} @ {} m/s, {} deg",
                fix.latitude_deg, fix.longitude_deg, fix.sog_mps, fix.cog_deg
            );
        } else {
            info!("gps fix: invalid (no fix or stale)");
        }
    }
}

/// REQ-001: reads the cup-and-vane sensor once per sample period, smooths
/// it (`Wind<N>`), and prints the live apparent-wind reading -- or that
/// it's currently invalid -- over RTT.
#[embassy_executor::task]
async fn wind_sensor_task(
    pulse_pin: ExtiInput<'static, embassy_stm32::mode::Async>,
    direction_adc: Adc<'static, peripherals::ADC1>,
    direction_pin: Peri<'static, peripherals::PA1>,
) {
    // Weather Meter Kit (SEN-15901) calibration -- direction_offset_deg
    // still needs confirming against actual mounting (tasks.md Task 1).
    let calibration = WindCalibration::weather_meter_kit_defaults();
    const SAMPLE_PERIOD_S: f32 = 1.0;

    let mut sensor = CupAndVaneWindSensor::new(pulse_pin, direction_adc, direction_pin, calibration, SAMPLE_PERIOD_S);
    let mut wind: Wind<4> = Wind::new();

    loop {
        let reading = sensor.sample().await;
        wind.update(reading);

        APPARENT_WIND.sender().send(WindReading {
            speed_mps: wind.filtered_speed(),
            direction_deg: wind.filtered_direction(),
            valid: wind.is_valid(),
        });

        if wind.is_valid() {
            info!(
                "apparent wind: {} m/s @ {} deg",
                wind.filtered_speed(),
                wind.filtered_direction()
            );
        } else {
            info!("apparent wind: invalid (sensor disconnected or unresponsive)");
        }
    }
}

#[embassy_executor::task]
async fn double_blinky_manually_assigned(
    _spawner: Spawner,
    pe1: Peri<'static, PE1>,
) {
    let mut led = Output::new(pe1, Level::High, Speed::Low);
    loop {
        info!("high");
        led.set_high();
        Timer::after_millis(500).await;

        info!("low");
        led.set_low();
        Timer::after_millis(500).await;
    }
}

#[embassy_executor::task]
pub async fn blink_red_led(
    _spawner: Spawner,
    pin: Peri<'static, PB14>,
    tim: Peri<'static, TIM12>,
) {
    let ch1_pin = PwmPin::new(pin, OutputType::PushPull);
    let mut pwm = SimplePwm::new(tim, Some(ch1_pin), None, None, None, khz(10), Default::default());
    let mut ch1: embassy_stm32::timer::simple_pwm::SimplePwmChannel<'_, TIM12> = pwm.ch1();
    ch1.enable();

    info!("PWM initialized");
    info!("PWM max duty {}", ch1.max_duty_cycle());

    loop {
        ch1.set_duty_cycle_fully_off();
        Timer::after_millis(300).await;
        ch1.set_duty_cycle_fraction(1, 4);
        Timer::after_millis(300).await;
        ch1.set_duty_cycle_fraction(1, 2);
        Timer::after_millis(300).await;
        ch1.set_duty_cycle(ch1.max_duty_cycle() - 1);
        Timer::after_millis(300).await;
    }
}
