#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use log::info;

use esp32c3_devkit_demo::{bsp::Board, led::write_led};
use smart_leds::colors::{BLACK, BLUE, GREEN, RED};

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) -> ! {
    esp_println::logger::init_logger_from_env();

    let mut board = Board::init();

    write_led(&mut board.led, BLUE, 50).unwrap();
    Timer::after_secs(1).await;
    write_led(&mut board.led, RED, 50).unwrap();
    Timer::after_secs(1).await;
    write_led(&mut board.led, GREEN, 50).unwrap();
    Timer::after_secs(1).await;
    write_led(&mut board.led, BLACK, 50).unwrap();

    // TODO: Spawn some tasks
    let _ = spawner;

    loop {
        info!("Hello world!");
        Timer::after(Duration::from_secs(1)).await;
    }
}
