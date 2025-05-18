#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use log::info;

use esp32c3_devkit_demo::{ambient, ble::GattServer, bsp::Board, led};
use smart_leds::colors::{BLUE, GREEN, RED};

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) -> ! {
    esp_println::logger::init_logger_from_env();
    let name = "Esp devkit demo";
    let board = Board::init();

    let (server, mut peripheral) = GattServer::start(name, spawner, board.ble_controller);

    let led_actor = led::spawn_actor(
        spawner,
        led::Config {
            led: board.led,
            ble: Some(server),
        },
    )
    .expect("failed to spawn led actor");
    led_actor.send(led::Message::SetBrightness(50)).await;
    let sequence = &[RED, GREEN, BLUE];
    led_actor
        .send(led::Message::SetSequence((
            sequence,
            Duration::from_secs(1),
            led::Repeat::Forever,
        )))
        .await;

    Timer::after(Duration::from_secs(1)).await;

    loop {
        info!("Hello world!");
        if let Ok(conn) = GattServer::advertise("Esp32c3-devkit-demo", &mut peripheral).await {
            // TODO
        }
    }
}
