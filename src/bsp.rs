//! Board Support Package for the esp32c3-rust-devkit board.
//!
//! <https://github.com/esp-rs/esp-rust-board>
//!
//! ### I2C Peripherals
//!
//! This board includes the following peripherals over the I2C bus:
//!
//! | Peripheral               | Part number | Reference                                                                                                      | Crate                                     | Address |
//! | ------------------------ | ----------- | -------------------------------------------------------------------------------------------------------------- | ----------------------------------------- | ------- |
//! | IMU                      | ICM-42670-P | [Datasheet](https://invensense.tdk.com/download-pdf/icm-42670-p-datasheet/)                                    | [Link](https://crates.io/crates/icm42670) | 0x68    |
//! | Temperature and Humidity | SHTC3       | [Datasheet](https://www.mouser.com/datasheet/2/682/Sensirion_04202018_HT_DS_SHTC3_Preliminiary_D2-1323493.pdf) | [Link](https://crates.io/crates/shtcx)    | 0x70    |
//!
//! #### I2C Bus Connection
//!
//! | Signal | GPIO   |
//! | ------ | ------ |
//! | SDA    | GPIO10 |
//! | SCL    | GPIO8  |
//!
//! ### I/Os
//!
//! The following devices are connected through GPIO:
//!
//! | I/O Devices | GPIO  |
//! | ----------- | ----- |
//! | WS2812 LED  | GPIO2 |
//! | LED         | GPIO7 |
//! | Button/Boot | GPIO9 |

use core::cell::RefCell;
use embassy_embedded_hal::shared_bus;
use embassy_sync::blocking_mutex::{NoopMutex, raw::NoopRawMutex};
use esp_hal::{
    clock::CpuClock,
    gpio::{Input, InputConfig, Pull},
    i2c::master::{Config, I2c},
    rmt::Rmt,
    rng::Rng,
    time::Rate,
    timer::systimer::SystemTimer,
};
use esp_hal_smartled::{SmartLedsAdapterAsync, buffer_size_async};

use esp_wifi::EspWifiController;
use log::info;
use static_cell::StaticCell;

use crate::{ble::BleController, led::Led};

pub type I2cType<'a> = I2c<'a, esp_hal::Async>;
pub type I2cBus<'a> = NoopMutex<RefCell<I2cType<'a>>>;
pub type I2cBusDevice<'a> = shared_bus::blocking::i2c::I2cDevice<'a, NoopRawMutex, I2cType<'a>>;

/// Board-specific peripherals.
pub struct Board {
    /// Onboard RGB LED
    pub led: Led,
    /// Random number generator
    pub rng: Rng,
    /// I2c Bus, shared between peripherals
    pub i2c_bus: &'static I2cBus<'static>,
    /// BLE controller
    pub ble_controller: BleController,
    /// Boot button
    pub button: Input<'static>,
}

impl Board {
    /// Initialize the board.
    pub fn init() -> Self {
        esp_println::logger::init_logger_from_env();

        let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
        let p = esp_hal::init(config);
        esp_alloc::heap_allocator!(size: 72 * 1024);

        info!("{} initialized!", esp_hal::chip!());

        let led = {
            let frequency = Rate::from_mhz(80);
            let rmt = Rmt::new(p.RMT, frequency)
                .expect("Failed to initialize RMT0")
                .into_async();
            SmartLedsAdapterAsync::new(rmt.channel0, p.GPIO2, [0; buffer_size_async(1)])
        };
        info!("Initialized WS2812 LED");

        let i2c_bus = {
            static BUS: StaticCell<I2cBus<'static>> = StaticCell::new();
            let i2c = I2c::new(p.I2C0, Config::default())
                .expect("Failed to initialize I2C0")
                .with_scl(p.GPIO8)
                .with_sda(p.GPIO10)
                .into_async();
            BUS.init(NoopMutex::new(RefCell::new(i2c)))
        };
        info!("Initialized I2C bus");

        let rng = Rng::new(p.RNG);

        let timer0 = SystemTimer::new(p.SYSTIMER);
        esp_hal_embassy::init(timer0.alarm0);
        info!("Initialized Embassy Executor");

        info!("Initializing BLE controller...");
        let controller: BleController = {
            let timg0 = esp_hal::timer::timg::TimerGroup::new(p.TIMG0);
            static WIFI: StaticCell<EspWifiController<'static>> = StaticCell::new();
            let init = WIFI.init(
                esp_wifi::init(timg0.timer0, rng, p.RADIO_CLK)
                    .expect("Failed to initialize BLE controller"),
            );
            let device = p.BT;
            let transport = esp_wifi::ble::controller::BleConnector::new(init, device);
            bt_hci::controller::ExternalController::new(transport)
        };
        let pull_up = InputConfig::default().with_pull(Pull::Up);
        Self {
            led,
            rng,
            i2c_bus,
            ble_controller: controller,
            button: Input::new(p.GPIO9, pull_up),
        }
    }
}
