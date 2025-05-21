//! # Ambient Sensor Example
//!
//! This example demonstrates how to read the temperature and humidity from the onboard SHTC3 sensor.
//! The SHTC3 sensor is a low-power sensor that can be used to measure temperature and humidity.
//! The sensor has a slow read time, so it is recommended to read the sensor no more than every 20 seconds.
//! This example demonstrates how to use an actor to read the sensor every 20 seconds.
//! The actor will read the sensor for 1 minute before stopping.

#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_futures::select::{Either, select};
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use esp32c3_devkit_demo::{ambient::AmbientSensor, bsp::Board};
use log::info;
use shtcx::PowerMode;

#[esp_hal_embassy::main]
async fn main(_spawner: Spawner) {
    esp_println::logger::init_logger_from_env();

    let board = Board::init();

    let mut ambient = AmbientSensor::new(board.i2c_bus);
    ambient
        .set_power_mode(PowerMode::NormalMode, Duration::from_millis(1000))
        .unwrap();
    // read the sensor for 10 seconds and then stop
    run_task(&mut ambient).await;

    ambient
        .set_power_mode(PowerMode::LowPower, Duration::from_millis(10))
        .unwrap();
    // read the sensor for 10 seconds and then stop
    run_task(&mut ambient).await;
}

/// Run the task to read the sensor for 10 seconds and then stop
/// This will read the sensor every 2 seconds and print the result.
/// If the sensor is not available, it will print an error message.
async fn run_task(sensor: &mut AmbientSensor) {
    let res = select(
        sensor.start_task(Duration::from_secs(2), None),
        Timer::after(Duration::from_secs(10)),
    )
    .await;
    if let Either::First(err) = res {
        info!("Error reading sensor: {:?}", err);
    };
}
