#![no_std]

use ector::mutex::NoopRawMutex;
use embassy_sync::channel::Sender;

pub mod ambient;
pub mod ble;
pub mod bsp;
pub mod buttons;
pub mod imu;
pub mod led;

/// Alias for the actor's inbox
pub type ActorInbox<M> = Sender<'static, NoopRawMutex, M, 1>;
