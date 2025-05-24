//! # Button Example
//!
//! This example demonstrates how to simply offload the handling of a button press to an async task.
//! In this example, we're using the onboard 'boot' button to toggle an LED on and off.
//! But in a more complex application, you could use this to trigger more complex actions.
//! Like sending a message to another actor.

#![no_std]
#![no_main]

use core::future::pending;
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_hal::gpio::Input;
use esp32c3_devkit_demo::{
    bsp::Board,
    led::{self, Led},
};
use smart_leds::colors::{BLACK, BLUE};

use esp_backtrace as _;

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) -> ! {
    let board = Board::init();

    spawner.must_spawn(button_task(board.button, board.led));

    pending().await
}

/// If we wanted multiple buttons to use this task, we could do so by
/// increasing the pool size of the task.
///
/// We can set up async tasks like this to handle button presses.
/// In this example we're just toggling an LED on and off.
/// But you could use this to trigger more complex actions.
/// Like sending a message to another actor.
#[embassy_executor::task(pool_size = 1)]
async fn button_task(mut button: Input<'static>, mut led: Led) {
    let debounce = Duration::from_millis(50);
    loop {
        button.wait_for_low().await;
        led::write(&mut led, BLUE, 30).await.unwrap();
        Timer::after(debounce).await;
        button.wait_for_high().await;
        led::write(&mut led, BLACK, 30).await.unwrap();
        Timer::after(debounce).await;
    }
}
