#![no_std]
#![no_main]

use embassy_futures::select::select3;
use embassy_time::{Duration, Timer};
use esp32c3_devkit_demo::{
    ambient::AmbientSensor,
    ble::GattServer,
    bsp::Board,
    imu::ImuSensor,
    led::{self, Repeat},
};
use icm42670::PowerMode as ImuMode;
use log::info;
use shtcx::PowerMode as AmbMode;
use smart_leds::colors::{BLUE, GREEN, RED};
use trouble_host::prelude::appearance;

use esp_backtrace as _;

#[esp_hal_embassy::main]
async fn main(spawner: embassy_executor::Spawner) -> ! {
    esp_println::logger::init_logger_from_env();
    let name = "Esp devkit demo";
    let appearance = &appearance::human_interface_device::GAMEPAD;
    let board = Board::init();

    let (server, mut peripheral) =
        GattServer::start(name, appearance, spawner, board.ble_controller);

    let led = led::spawn_actor(spawner, board.led).expect("failed to spawn led actor");
    led.set_brightness(50);
    let sequence = &[RED, GREEN, BLUE];

    let mut imu = ImuSensor::new(board.i2c_bus);
    let mut ambient = AmbientSensor::new(board.i2c_bus);
    Timer::after(Duration::from_secs(1)).await;

    loop {
        info!("Advertising for BLE Connection...");
        led.set_sequence(sequence, Duration::from_secs(1), Repeat::Forever);
        let Ok(conn) = server
            .advertise("Esp32c3-devkit-demo", &mut peripheral)
            .await
        else {
            continue;
        };
        led.off();
        let ble = (server, &conn);
        imu.set_power_mode(ImuMode::SixAxisLowNoise)
            .expect("sensor available");
        ambient.set_power_mode(AmbMode::LowPower, Duration::from_millis(100));

        let imu_task = imu.start_task(Duration::from_hz(20), Some(ble));
        let amb_task = ambient.start_task(Duration::from_hz(1), Some(ble));
        let gatt_task = server.start_task(&conn);
        select3(imu_task, amb_task, gatt_task).await;
    }
}
