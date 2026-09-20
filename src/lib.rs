#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]

pub mod windmeter;
pub mod moving_average;
pub(crate) mod angle;
pub mod wind_sensor;
pub mod gps;
pub mod wind_compute;
pub mod logger;
pub mod nmea2000;