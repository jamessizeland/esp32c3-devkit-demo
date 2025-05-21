use trouble_host::prelude::*;

#[gatt_service(uuid = service::ENVIRONMENTAL_SENSING)]
pub struct AmbientService {
    #[descriptor(uuid = descriptors::MEASUREMENT_DESCRIPTION, read, value = "Temperature °C")]
    #[characteristic(uuid = characteristic::TEMPERATURE, read, notify)]
    pub temperature: i16,
    #[descriptor(uuid = descriptors::MEASUREMENT_DESCRIPTION, read, value = "Humidity %")]
    #[characteristic(uuid = characteristic::HUMIDITY, read, notify)]
    pub humidity: i16,
}

#[gatt_service(uuid = "911fd452-297b-408f-8f53-ada4e57647dd")]
pub struct AccelerationService {
    #[descriptor(uuid = descriptors::MEASUREMENT_DESCRIPTION, read, value = "Accelerometer X m/s²")]
    #[characteristic(uuid = "17bc0927-4de9-4d62-b234-7e1bde9f0c60", read, notify)]
    pub x: f32,
    #[descriptor(uuid = descriptors::MEASUREMENT_DESCRIPTION, read, value = "Accelerometer Y m/s²")]
    #[characteristic(uuid = "17bc0927-4de9-4d62-b234-7e1bde9f0c61", read, notify)]
    pub y: f32,
    #[descriptor(uuid = descriptors::MEASUREMENT_DESCRIPTION, read, value = "Accelerometer Z m/s²")]
    #[characteristic(uuid = "17bc0927-4de9-4d62-b234-7e1bde9f0c62", read, notify)]
    pub z: f32,
}

#[gatt_service(uuid = "911fd452-297b-408f-8f53-ada4e57647dc")]
pub struct GyroscopeService {
    #[descriptor(uuid = descriptors::MEASUREMENT_DESCRIPTION, read, value = "Gyroscope X °/s")]
    #[characteristic(uuid = "17bc0927-4de9-4d62-b234-7e1bde9f0c63", read, notify)]
    pub x: f32,
    #[descriptor(uuid = descriptors::MEASUREMENT_DESCRIPTION, read, value = "Gyroscope Y °/s")]
    #[characteristic(uuid = "17bc0927-4de9-4d62-b234-7e1bde9f0c64", read, notify)]
    pub y: f32,
    #[descriptor(uuid = descriptors::MEASUREMENT_DESCRIPTION, read, value = "Gyroscope Z °/s")]
    #[characteristic(uuid = "17bc0927-4de9-4d62-b234-7e1bde9f0c65", read, notify)]
    pub z: f32,
}
#[gatt_service(uuid = "911fd452-297b-408f-8f53-ada4e57647dd")]
pub struct InclinationService {
    #[descriptor(uuid = descriptors::MEASUREMENT_DESCRIPTION, read, value = "Inclination X °")]
    #[characteristic(uuid = "17bc0927-4de9-4d62-b234-7e1bde9f0c66", read, notify)]
    pub x: f32,
    #[descriptor(uuid = descriptors::MEASUREMENT_DESCRIPTION, read, value = "Inclination Y °")]
    #[characteristic(uuid = "17bc0927-4de9-4d62-b234-7e1bde9f0c67", read, notify)]
    pub y: f32,
    #[descriptor(uuid = descriptors::MEASUREMENT_DESCRIPTION, read, value = "Inclination Z °")]
    #[characteristic(uuid = "17bc0927-4de9-4d62-b234-7e1bde9f0c68", read, notify)]
    pub z: f32,
}

#[gatt_service(uuid = "911fd452-297b-408f-8f53-ada4e57647de")]
pub struct HidService {
    #[characteristic(uuid = characteristic::BOOLEAN, read, notify)]
    pub state: bool,
}

#[gatt_server]
pub struct GattServer {
    pub ambient: AmbientService,
    pub acceleration: AccelerationService,
    pub gyroscope: GyroscopeService,
    pub inclination: InclinationService,
    pub hid: HidService,
}
