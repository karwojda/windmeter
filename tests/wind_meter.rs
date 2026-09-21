#![cfg(feature = "embedded")]
#![no_std]
#![no_main]

use defmt_rtt as _;

#[embedded_test::tests]
mod tests {
    use embassy_time::{Duration, Timer};

    // The optional init function runs before EVERY test case.
    // It handles the critical hardware configuration.
    #[init]
    async fn init() -> embassy_stm32::Peripherals {
        // Initialize the Embassy STM32 HAL
        embassy_stm32::init(Default::default())
    }

    // A fully async integration test case
    #[test]
    async fn test_async_delay(mut p: embassy_stm32::Peripherals) {
        let start = embassy_time::Instant::now();
        
        // Test an async operation (like an embassy timer)
        Timer::after(Duration::from_millis(100)).await;
        
        let elapsed = start.elapsed().as_millis();
        assert!(elapsed >= 100);
    }
}