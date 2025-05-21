#![no_std]

use ector::mutex::NoopRawMutex;
use embassy_sync::channel::Sender;
use esp_hal_smartled::LedAdapterError;
use thiserror::Error;

pub mod ambient;
pub mod ble;
pub mod bsp;
pub mod buttons;
pub mod imu;
pub mod led;

/// Alias for the actor's inbox
pub type ActorInbox<M> = Sender<'static, NoopRawMutex, M, 10>;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Failed to write to LED: {0:?}")]
    LedWrite(LedAdapterError),
    #[error("Failed to send message to LED actor")]
    LedActorSend,
    #[error("Failed to send message to IMU actor")]
    ImuActorSend,
    #[error("Read time {0}ms must be less than max read time {1}ms")]
    InvalidReadTime(u64, u64),
    #[error("Read period {0}ms must be greater than read time {1}ms")]
    InvalidReadPeriod(u64, u64),
    #[error("Failed to read from Ambient Sensor")]
    AmbientI2cRead,
}
