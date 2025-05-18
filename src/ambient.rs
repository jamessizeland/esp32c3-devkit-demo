//! A simple struct to read from the Ambient Sensor.

use crate::bsp::I2cBusDevice;
use crate::{ble::GattServer, bsp::I2cBus};
use embassy_embedded_hal::shared_bus::{I2cDeviceError, blocking::i2c::I2cDevice};
use embassy_time::Instant;
use embassy_time::{Duration, Timer};
use esp_hal::i2c::master::Error;
use log::info;
use shtcx::ShtC3;
use shtcx::{Measurement, PowerMode};
use trouble_host::gatt::GattConnection;

pub struct AmbientSensor {
    /// The Onboard temperature and humidity sensor
    device: ShtC3<I2cBusDevice<'static>>,
}

impl AmbientSensor {
    /// Initialize the sensor.
    pub fn new(i2c_bus: &'static I2cBus<'static>) -> Self {
        let i2c = I2cDevice::new(i2c_bus);
        Self {
            device: shtcx::shtc3(i2c),
        }
    }
    /// Start reading the sensor at a given period in Normal Power Mode.
    ///
    /// Optionally Notify the BLE client with the latest measurement.
    ///
    /// The period must be greater than the read time, which is
    /// the time it takes to read from the sensor in the current power mode.
    pub async fn read(
        &mut self,
        period: Duration,
        ble: Option<(&'_ GattServer<'_>, &GattConnection<'_, '_>)>,
    ) {
        self.read_inner(period, PowerMode::NormalMode, ble).await;
    }

    /// Start reading the sensor at a given period in Low Power Mode.
    ///
    /// Optionally Notify the BLE client with the latest measurement.
    ///
    /// The period must be greater than the read time, which is
    /// the time it takes to read from the sensor in the current power mode.
    pub async fn read_low_power(
        &mut self,
        period: Duration,
        ble: Option<(&'_ GattServer<'_>, &GattConnection<'_, '_>)>,
    ) {
        self.read_inner(period, PowerMode::LowPower, ble).await;
    }
}

impl AmbientSensor {
    /// Start reading the sensor at a given period in a given power mode.
    async fn read_inner(
        &mut self,
        period: Duration,
        power_mode: PowerMode,
        ble: Option<(&'_ GattServer<'_>, &GattConnection<'_, '_>)>,
    ) {
        let read_time = {
            let val = shtcx::max_measurement_duration(&self.device, power_mode);
            Duration::from_millis(val.into())
        };
        info!("Read time: {:?} seconds", read_time.as_secs());
        info!("Starting measurement every {:?} seconds", period.as_secs());
        assert!(period > read_time, "Period must be greater than read time");
        loop {
            let now = Instant::now();
            match self.read_measurement(read_time, power_mode).await {
                Ok(meas) => {
                    if let Some(ble) = ble {
                        if let Err(error) = self.notify_ble(ble, meas).await {
                            log::error!("Error notifying BLE: {:?}", error);
                        }
                    } else {
                        info!("Temperature: {:?}", meas.temperature.as_degrees_celsius());
                        info!("Humidity: {:?}", meas.humidity.as_percent());
                    }
                }
                Err(error) => {
                    log::error!("ambient error: {:?}", error);
                    return;
                }
            };
            Timer::after(period - now.elapsed()).await;
        }
    }

    /// Notify the BLE client with the latest measurement
    async fn notify_ble(
        &self,
        ble: (&GattServer<'_>, &GattConnection<'_, '_>),
        measurement: Measurement,
    ) -> Result<(), trouble_host::Error> {
        let (amb, conn) = (&ble.0.ambient, ble.1);
        amb.humidity
            .notify(conn, &measurement.humidity.as_percent())
            .await?;
        amb.temperature
            .notify(conn, &measurement.temperature.as_degrees_celsius())
            .await
    }

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
