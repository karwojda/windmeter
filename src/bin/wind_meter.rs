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
use embassy_stm32::{bind_interrupts, exti, peripherals, Config, Peri};
use embassy_time::Timer;
use embassy_stm32::peripherals::{PE1, PB14, TIM12};
use {defmt_rtt as _, panic_probe as _};
use wind_meter::wind_sensor::{CupAndVaneWindSensor, WindCalibration};
use wind_meter::windmeter::Wind;

// TODO: placeholder pin assignment -- confirm against the real anemometer
// wiring during the hardware verification pass (tasks.md Task 1) and
// update here. Anything downstream of `WindCalibration` is unaffected by
// which pins these turn out to be.
bind_interrupts!(struct Irqs {
    EXTI0 => exti::InterruptHandler<ExtiLine0>;
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
    // TODO: mps_per_hz / direction_offset_deg are placeholders -- confirm
    // against the real sensor's datasheet and mounting during hardware
    // verification (tasks.md Task 1).
    let calibration = WindCalibration {
        mps_per_hz: 0.5,
        direction_offset_deg: 0.0,
        adc_max: 4095,
        stale_after_samples: 5,
    };
    const SAMPLE_PERIOD_S: f32 = 1.0;

    let mut sensor = CupAndVaneWindSensor::new(pulse_pin, direction_adc, direction_pin, calibration, SAMPLE_PERIOD_S);
    let mut wind: Wind<4> = Wind::new();

    loop {
        let reading = sensor.sample().await;
        wind.update(reading);

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
