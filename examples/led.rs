//! # LED Example
//!
//! This example demonstrates how to control the onboard LED using an actor.
//! The actor will control the LED asynchronously.
//! The actor will run a sequence of colours forever.

#![no_std]
#![no_main]

use core::future::pending;

use esp_backtrace as _;

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use smart_leds::colors::{BLUE, GREEN, RED, YELLOW};

use esp32c3_devkit_demo::{
    bsp::Board,
    led::{self, Repeat},
};

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) -> ! {
    esp_println::logger::init_logger_from_env();

    let mut board = Board::init();

    // Can write to the LED directly.
    led::write(&mut board.led, BLUE, 50).await.unwrap();
    Timer::after_secs(1).await;

    // Can also spawn an actor to control the LED asynchronously.
    // The actor inbox can be shared with other actors to send messages to this actor.
    let led = led::spawn_actor(spawner, board.led).expect("failed to spawn led actor");

    led.set_brightness(50).unwrap();
    led.set_colour(YELLOW).unwrap();
    Timer::after_secs(1).await;

    // This sequence will run forever until the actor is dropped, or another message is sent.
    // It will run as a background task.
    let sequence = &[RED, GREEN, BLUE];
    led.set_sequence(sequence, Duration::from_secs(1), Repeat::Forever)
        .unwrap();

    pending().await
}
