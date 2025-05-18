#![no_std]
#![no_main]

use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use log::info;

use esp32c3_devkit_demo::{
    ble::GattServer,
    bsp::Board,
    led::{self, Repeat},
};
use smart_leds::colors::{BLUE, GREEN, RED};

#[esp_hal_embassy::main]
async fn main(spawner: embassy_executor::Spawner) -> ! {
    esp_println::logger::init_logger_from_env();
    let name = "Esp devkit demo";
    let board = Board::init();

    let (server, mut peripheral) = GattServer::start(name, spawner, board.ble_controller);

    let led = led::spawn_actor(spawner, board.led).expect("failed to spawn led actor");
    led.set_brightness(50);
    let sequence = &[RED, GREEN, BLUE];
    led.set_sequence(sequence, Duration::from_secs(1), Repeat::Forever);

    Timer::after(Duration::from_secs(1)).await;

    loop {
        info!("Hello world!");
        if let Ok(conn) = GattServer::advertise("Esp32c3-devkit-demo", &mut peripheral).await {}
    }
}
