#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_stm32::gpio::{Level, Output, OutputType, Speed};
use embassy_stm32::time::khz;
use embassy_stm32::timer::simple_pwm::{*,PwmPin};
use embassy_stm32::{Config, Peri};
use embassy_time::Timer;
use embassy_stm32::peripherals::{PE1, PB14, TIM12};
use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let config = Config::default();

    let p = embassy_stm32::init(config);
    info!("Hello World!");

    let _ = spawner.spawn(double_blinky_manually_assigned(spawner, p.PE1));
    
    let _ = spawner.spawn(blink_red_led(spawner, p.PB14, p.TIM12));

    
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
