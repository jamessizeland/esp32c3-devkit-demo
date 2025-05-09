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

use esp32c3_devkit_demo::{
    bsp::Board,
    imu::{self, Message},
};

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) -> ! {
    esp_println::logger::init_logger_from_env();

    let board = Board::init();

    // For now we only have one element in the configuration.
    // But we could add more elements to the configuration to pass to the actor.
    let config = imu::Config {
        i2c_bus: board.i2c_bus,
    };
    let actor = imu::spawn_actor(spawner, config).expect("failed to spawn");

    // Set the power mode to normal mode.
    actor
        .send(Message::SetPowerMode(icm42670::PowerMode::SixAxisLowNoise))
        .await;

    Timer::after_secs(1).await;

    // Start the actor to read the sensor every 20 milliseconds.
    actor.send(Message::Start(Duration::from_millis(20))).await;

    // Stop the actor after 60 seconds.
    Timer::after_secs(60).await;
    actor.send(Message::Stop).await;

    pending().await
}
