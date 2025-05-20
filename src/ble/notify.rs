use super::GattServer;

impl GattServer<'_> {
    /// Notify the BLE central with the latest IMU data.
    pub async fn notify_imu(
        &self,
        conn: &trouble_host::gatt::GattConnection<'_, '_>,
        measurement: crate::imu::Measurement,
    ) -> Result<(), trouble_host::Error> {
        let accel = measurement.accel;
        let gyro = measurement.gyro;
        let incl = measurement.inclination;
        self.acceleration.x.notify(conn, &accel.x).await?;
        self.acceleration.y.notify(conn, &accel.y).await?;
        self.acceleration.z.notify(conn, &accel.z).await?;
        self.gyroscope.x.notify(conn, &gyro.x).await?;
        self.gyroscope.y.notify(conn, &gyro.y).await?;
        self.gyroscope.z.notify(conn, &gyro.z).await?;
        if let Some(incl) = incl {
            self.inclination.x.notify(conn, &incl.x).await?;
            self.inclination.y.notify(conn, &incl.y).await?;
            self.inclination.z.notify(conn, &incl.z).await?;
        }
        Ok(())
    }
    /// Notify the BLE central with the latest Temperature and Humidity data.
    pub async fn notify_ambient(
        &self,
        conn: &trouble_host::gatt::GattConnection<'_, '_>,
        measurement: shtcx::Measurement,
    ) -> Result<(), trouble_host::Error> {
        self.ambient
            .humidity
            .notify(conn, &measurement.humidity.as_percent())
            .await?;
        self.ambient
            .temperature
            .notify(conn, &measurement.temperature.as_degrees_celsius())
            .await
    }
}
