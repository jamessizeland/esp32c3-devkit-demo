//! A simple struct to read from the Ambient Sensor.

use crate::ble::BleConnection;
use crate::bsp::I2cBus;
use crate::bsp::I2cBusDevice;
use embassy_embedded_hal::shared_bus::{I2cDeviceError, blocking::i2c::I2cDevice};
use embassy_time::Instant;
use embassy_time::{Duration, Timer};
use esp_hal::i2c::master::Error;
use log::info;
use shtcx::ShtC3;
use shtcx::{Measurement, PowerMode};

pub struct AmbientSensor {
    /// The Onboard temperature and humidity sensor
    device: ShtC3<I2cBusDevice<'static>>,
    /// The power mode of the sensor
    power_mode: PowerMode,
    /// Length of time to take a sample
    read_time: Duration,
}

impl AmbientSensor {
    /// Initialize the sensor.
    pub fn new(i2c_bus: &'static I2cBus<'static>) -> Self {
        let i2c = I2cDevice::new(i2c_bus);
        Self {
            device: shtcx::shtc3(i2c),
            power_mode: PowerMode::LowPower,
            read_time: Duration::from_millis(100),
        }
    }
    /// Set the power mode of the sensor.
    pub fn set_power_mode(&mut self, power_mode: PowerMode, read_time: Duration) {
        let max_read_time = {
            let val = shtcx::max_measurement_duration(&self.device, power_mode);
            Duration::from_millis(val.into())
        };
        info!("Max read time: {:?} ms", max_read_time.as_millis());
        info!("Read time: {:?} ms", read_time.as_millis());
        assert!(
            max_read_time >= read_time,
            "Read time ({}ms) must be less than {}ms",
            read_time.as_millis(),
            max_read_time.as_millis()
        );
        self.power_mode = power_mode;
        self.read_time = read_time;
    }

    /// Start reading the sensor at a given period in set Power Mode.
    ///
    /// Optionally Notify the BLE client with the latest measurement.
    ///
    /// The period must be greater than the read time, which is
    /// the time it takes to read from the sensor in the current power mode.
    pub async fn start_task(
        &mut self,
        period: Duration,
        ble: Option<BleConnection<'_, '_, '_>>,
    ) -> Result<(), shtcx::Error<I2cDeviceError<Error>>> {
        assert!(
            period >= self.read_time,
            "Period must be greater than read time"
        );
        info!("Taking measurement every {:?} seconds", period.as_secs());
        loop {
            let now = Instant::now();
            let meas = self
                .read_measurement(self.read_time, self.power_mode)
                .await?;
            if let Some((server, conn)) = ble {
                if let Err(error) = server.notify_ambient(conn, meas).await {
                    log::error!("Error notifying BLE: {:?}", error);
                }
            } else {
                info!("Temperature: {:?}°C", meas.temperature.as_degrees_celsius());
                info!("Humidity: {:?}%RH", meas.humidity.as_percent());
            }
            Timer::after(period.checked_sub(now.elapsed()).unwrap_or_default()).await;
        }
    }
}

impl AmbientSensor {
    /// Read the temperature and humidity from the sensor
    async fn read_measurement(
        &mut self,
        read_time: Duration,
        power_mode: PowerMode,
    ) -> Result<Measurement, shtcx::Error<I2cDeviceError<Error>>> {
        self.device.start_measurement(power_mode)?;
        Timer::after(read_time).await;
        self.device.get_measurement_result()
    }
}
