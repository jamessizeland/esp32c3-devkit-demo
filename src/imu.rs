//! A simple struct to read from the 6-axis IMU.
//!
//! This can be used to read from the onboard IMU sensor.
//! at a set rate or on demand.

use embassy_embedded_hal::shared_bus::I2cDeviceError;
use embassy_embedded_hal::shared_bus::blocking::i2c::I2cDevice;
use embassy_time::{Duration, Instant, Timer};
use esp_hal::i2c::master::Error as I2cError;
use gimbal::Gimbal;
use icm42670::accelerometer::Error as AccelError;
use icm42670::accelerometer::vector::F32x3;
use icm42670::{Address, prelude::*};
use icm42670::{Icm42670, PowerMode};
use log::info;

use crate::ble::BleConnection;
use crate::bsp::{I2cBus, I2cBusDevice};

pub struct ImuSensor {
    /// The Onboard gyroscope and accelerometer.
    device: Icm42670<I2cBusDevice<'static>>,
    /// The gimbal to calculate inclination
    gimbal: Option<Gimbal>,
    /// The power mode of the sensor
    power_mode: PowerMode,
}

pub struct Measurement {
    /// 3 axis acceleration
    pub accel: F32x3,
    /// 3 axis gyroscope
    pub gyro: F32x3,
    /// 3 axis inclination
    pub inclination: Option<F32x3>,
}

impl ImuSensor {
    /// Create a new actor with a spawner and a configuration.
    pub fn new(i2c_bus: &'static I2cBus<'static>) -> Self {
        let i2c = I2cDevice::new(i2c_bus);
        let mut device =
            Icm42670::new(i2c, Address::Primary).expect("Failed to initialize ICM42670");
        let power_mode = PowerMode::Standby;
        device
            .set_power_mode(power_mode)
            .expect("Failed to set power mode");
        info!("Sample rate is: {:?}", device.sample_rate());
        device.soft_reset().expect("Failed to reset device");
        Self {
            gimbal: None,
            device,
            power_mode,
        }
    }
    /// Set the power mode of the sensor.
    pub fn set_power_mode(
        &mut self,
        power_mode: PowerMode,
    ) -> Result<(), icm42670::Error<I2cDeviceError<I2cError>>> {
        self.power_mode = power_mode;
        self.device.set_power_mode(power_mode)
    }
    /// Start reading the sensor at a given period.
    ///
    /// Optionally Notify the BLE client with the latest measurement.
    pub async fn start_task(
        &mut self,
        period: Duration,
        ble: Option<BleConnection<'_, '_>>,
    ) -> Result<(), AccelError<icm42670::Error<I2cDeviceError<I2cError>>>> {
        self.read_inner(period, ble).await
    }

    /// Read the accelerometer and gyroscope from the sensor.
    ///
    /// Calculate the inclination if a gymbal has been set up.
    pub async fn read_measurement(
        &mut self,
    ) -> Result<Measurement, AccelError<icm42670::Error<I2cDeviceError<I2cError>>>> {
        let accel = self.device.accel_norm()?;
        let gyro = self.device.gyro_norm()?;
        let inclination = self.gimbal.as_mut().map(|g| g.read(gyro, accel));
        Ok(Measurement {
            accel,
            gyro,
            inclination,
        })
    }
}

impl ImuSensor {
    /// Start reading the sensor at a given period.
    async fn read_inner(
        &mut self,
        period: Duration,
        ble: Option<BleConnection<'_, '_>>,
    ) -> Result<(), AccelError<icm42670::Error<I2cDeviceError<I2cError>>>> {
        self.gimbal = Some(Gimbal::new(period));
        info!(
            "Starting measurement every {:?} milliseconds",
            period.as_millis()
        );
        let max_rate = self.device.sample_rate()?;

        let read_time = Duration::from_secs(1) / max_rate as u32;
        assert!(period > read_time, "Period must be greater than read time");
        loop {
            let now = Instant::now();
            let meas = self.read_measurement().await?;
            if let Some((server, conn)) = ble {
                if let Err(error) = server.notify_imu(conn, meas).await {
                    log::error!("Error notifying BLE: {:?}", error);
                }
            } else {
                log::info!("Gyro: {:?}", meas.gyro);
                log::info!("Accel: {:?}", meas.accel);
                log::info!("Inclination: {:?}", meas.inclination);
            }
            Timer::after(period - now.elapsed()).await;
        }
    }
}

mod gimbal {
    use embassy_time::{Duration, Instant};
    use icm42670::accelerometer::vector::F32x3;
    use imu_fusion::{Fusion, FusionAhrsSettings, FusionQuaternion, FusionVector};
    use micromath::F32Ext;

    pub struct Gimbal(Fusion);

    impl Gimbal {
        /// Create a new gimbal with a period between measurements
        pub fn new(period: Duration) -> Self {
            Self(Fusion::new(
                period.as_millis() as u32,
                FusionAhrsSettings::new(),
            ))
        }

        /// Read the sensor data and calculate the inclination
        pub fn read(&mut self, gyro: F32x3, accel: F32x3) -> F32x3 {
            let now = Instant::now();
            // our sensor does not have a magnetometer
            self.0.update_no_mag(
                FusionVector::new(gyro.x, gyro.y, gyro.z),
                FusionVector::new(accel.x, accel.y, accel.z),
                (now.as_millis() as f64 / 1000.0) as f32,
            );
            let quaternion = self.0.quaternion();
            let fusion_accel = Self::quaternion_to_acceleration(&quaternion);
            Self::inclination(fusion_accel)
        }

        /// Calculate the inclination of the sensor in degrees
        fn inclination(accel: F32x3) -> F32x3 {
            let F32x3 { x, y, z } = accel;
            let (x_sq, y_sq, z_sq) = (x.powi(2), y.powi(2), z.powi(2));
            F32x3 {
                x: (x.atan2((y_sq + z_sq).sqrt())).to_degrees(),
                y: (y.atan2((x_sq + z_sq).sqrt())).to_degrees(),
                z: (z.atan2((x_sq + y_sq).sqrt())).to_degrees(),
            }
        }

        /// Convert a quaternion to a 3D acceleration vector.
        fn quaternion_to_acceleration(quaternion: &FusionQuaternion) -> F32x3 {
            let FusionQuaternion { x, y, z, w } = quaternion;
            F32x3 {
                x: 2.0 * (x * z - w * y),
                y: 2.0 * (y * z + w * x),
                z: w.powi(2) - x.powi(2) - y.powi(2) + z.powi(2),
            }
        }
    }
}
