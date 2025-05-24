//! # IMU Sensor Example
//!
//! This example demonstrates how to read the accelerometer and gyroscope data from the onboard ICM42670 sensor.
//! The ICM42670 sensor is a low-power sensor that can be used to measure acceleration and angular velocity.
//! This example demonstrates how to use an actor to read the sensor every 20 milliseconds.

#![no_std]
#![no_main]

use core::future::pending;
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp32c3_devkit_demo::{bsp::Board, imu::ImuSensor};

use esp_backtrace as _;

#[esp_hal_embassy::main]
async fn main(_spawner: Spawner) -> ! {
    let board = Board::init();

    let mut imu = ImuSensor::new(board.i2c_bus);

    // Set the power mode to normal mode.
    imu.set_power_mode(icm42670::PowerMode::SixAxisLowNoise)
        .unwrap();

    Timer::after_secs(1).await;

    // Start the imu to read the sensor every 20 milliseconds.
    imu.start_task(Duration::from_millis(20), None)
        .await
        .unwrap();

    pending().await
}
