//! # Ambient Sensor Example
//!
//! This example demonstrates how to read the temperature and humidity from the onboard SHTC3 sensor.
//! The SHTC3 sensor is a low-power sensor that can be used to measure temperature and humidity.
//! The sensor has a slow read time, so it is recommended to read the sensor no more than every 20 seconds.
//! This example demonstrates how to use an actor to read the sensor every 20 seconds.
//! The actor will read the sensor for 1 minute before stopping.

#![no_std]
#![no_main]

use embassy_futures::select::select;
use esp_backtrace as _;

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};

use esp32c3_devkit_demo::{ambient::AmbientSensor, bsp::Board};

#[esp_hal_embassy::main]
async fn main(_spawner: Spawner) {
    esp_println::logger::init_logger_from_env();

    let board = Board::init();

    let mut ambient = AmbientSensor::new(board.i2c_bus);

    // select(
    //     // read the sensor every 2 seconds
    //     ambient.read(Duration::from_secs(1), Duration::from_secs(5), None),
    //     // for 10 seconds
    //     Timer::after(Duration::from_secs(60)),
    // )
    // .await;
    select(
        // read the sensor every 2 seconds
        ambient.read_low_power(Duration::from_millis(800), Duration::from_millis(800), None),
        // for 10 seconds
        Timer::after(Duration::from_secs(10)),
    )
    .await;
}
