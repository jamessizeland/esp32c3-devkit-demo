//! # IMU Sensor Example
//!
//! This example demonstrates how to read the accelerometer and gyroscope data from the onboard ICM42670 sensor.
//! The ICM42670 sensor is a low-power sensor that can be used to measure acceleration and angular velocity.
//! This example demonstrates how to use an actor to read the sensor every 20 seconds.
//! The actor will read the sensor for 1 minute before stopping.

#![no_std]
#![no_main]

use core::future::pending;

use esp_backtrace as _;

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};

use esp32c3_devkit_demo::{bsp::Board, imu};

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) -> ! {
    esp_println::logger::init_logger_from_env();

    let board = Board::init();

    let imu = imu::spawn_actor(spawner, board.i2c_bus).expect("failed to spawn");

    // Set the power mode to normal mode.
    imu.set_power_mode(icm42670::PowerMode::SixAxisLowNoise);

    Timer::after_secs(1).await;

    // Start the imu to read the sensor every 20 milliseconds.
    imu.start(Duration::from_millis(20));

    // Stop the imu after 60 seconds.
    Timer::after_secs(60).await;
    imu.stop();

    pending().await
}
