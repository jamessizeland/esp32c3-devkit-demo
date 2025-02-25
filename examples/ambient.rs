#![no_std]
#![no_main]

use core::future::pending;

use esp_backtrace as _;

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};

use esp32c3_devkit_demo::{ambient, bsp::Board};

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) -> ! {
    esp_println::logger::init_logger_from_env();

    let board = Board::init();

    let actor = ambient::spawn_actor(
        spawner,
        ambient::Config {
            i2c_bus: board.i2c_bus,
        },
    )
    .expect("failed to spawn ambient actor");
    actor
        .send(ambient::Message::SetPowerMode(shtcx::PowerMode::NormalMode))
        .await;
    Timer::after_secs(1).await;

    actor
        .send(ambient::Message::Start(Duration::from_secs(20)))
        .await;

    Timer::after_secs(60).await;
    actor.send(ambient::Message::Stop).await;

    pending().await
}
