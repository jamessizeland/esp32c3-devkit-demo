//! A simple actor to read from the 6-axis IMU.
//!
//! This actor can be used to read from the onboard IMU sensor.
//! at a set rate or on demand.

use actor_private::*;
use ector::ActorContext;
use embassy_executor::Spawner;
use gimbal::Gimbal;
use icm42670::PowerMode;
use log::info;
use {
    core::future::pending,
    embassy_executor::SpawnError,
    embassy_futures::select::{Either, select},
    embassy_time::{Duration, Timer},
};

use crate::{ActorInbox, bsp::I2cBus};

/// The actor's message type, communicating the finite states of the actor.
/// This is made available to other actors to interact with this one.
pub enum Message {
    /// Set the power mode of the sensor
    SetPowerMode(PowerMode),
    /// Read the data from the sensor at a set period
    Start(Duration),
    /// Stop reading the data from the sensor
    Stop,
}

/// The actor's configuration, to be shared with other actors to initialize this actor.
pub struct Config {
    pub i2c_bus: &'static I2cBus<'static>,
}

/// Create a new actor with a spawner and a configuration.
pub fn spawn_actor(spawner: Spawner, config: Config) -> Result<ActorInbox<Message>, SpawnError> {
    static CONTEXT: ActorContext<Actor> = ActorContext::new();
    let inbox = CONTEXT.address();
    spawner.spawn(actor_task(&CONTEXT, Actor::new(spawner, config, inbox)))?;
    Ok(inbox)
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

mod actor_private {

    use ector::{DynamicAddress, Inbox};
    use embassy_embedded_hal::shared_bus::blocking::i2c::I2cDevice;
    use embassy_time::Instant;
    use icm42670::{Address, Icm42670, prelude::*};

    use crate::bsp::I2cBusDevice;

    use super::*;
    /// A scheduler to run a sequence of actions.
    struct Scheduler {
        /// The timer to schedule the next action
        timer: Timer,
        /// The period between actions
        period: Duration,
    }

    /// The actor's private data, not to be shared with other actors.
    /// This is where the actor's state is stored.
    pub(super) struct Actor {
        /// A timer to schedule the next message
        scheduler: Option<Scheduler>,
        /// The Onboard temperature and humidity sensor
        device: Icm42670<I2cBusDevice<'static>>,
        /// The current power mode of the sensor
        power_mode: PowerMode,
        /// The gimbal to calculate the inclination
        gimbal: Option<Gimbal>,
        /// Error state
        error: bool,
    }

    impl ector::Actor for Actor {
        type Message = Message;

        /// Actor pattern for either handling new incoming messages or running a scheduled action.
        async fn on_mount<M>(&mut self, _: DynamicAddress<Message>, mut inbox: M) -> !
        where
            M: Inbox<Self::Message>,
        {
            info!("Ambient Task started!");
            loop {
                let deadline = async {
                    match self.scheduler.as_mut() {
                        Some(Scheduler { timer, .. }) => timer.await,
                        None => pending().await,
                    }
                };
                match select(inbox.next(), deadline).await {
                    Either::First(action) => self.act(action).await,
                    Either::Second(_) => self.next().await,
                }
            }
        }
    }

    impl Actor {
        /// Create a new actor with a spawner and a configuration.
        pub(super) fn new(_: Spawner, config: Config, _: ActorInbox<Message>) -> Self {
            let i2c = I2cDevice::new(config.i2c_bus);
            let mut device =
                Icm42670::new(i2c, Address::Primary).expect("Failed to initialize ICM42670");
            let power_mode = PowerMode::Standby;
            device
                .set_power_mode(power_mode)
                .expect("Failed to set power mode");
            info!("Sample rate is: {:?}", device.sample_rate());
            device.soft_reset().expect("Failed to reset device");
            Self {
                scheduler: None,
                gimbal: None,
                device,
                power_mode,
                error: false,
            }
        }
        /// The message handler
        async fn act(&mut self, msg: Message) {
            match msg {
                Message::SetPowerMode(power_mode) => {
                    self.power_mode = power_mode;
                    if self.device.set_power_mode(power_mode).is_err() {
                        log::error!("Failed to set power mode");
                        self.error = true;
                        return;
                    };
                    info!("Power mode set to {:?}", power_mode);
                }
                Message::Start(period) => {
                    self.gimbal = Some(Gimbal::new(period));
                    info!("Starting measurement every {:?} seconds", period.as_secs());
                    let Ok(max_rate) = self.device.sample_rate() else {
                        log::error!("Failed to get sample rate");
                        self.error = true;
                        return;
                    };
                    let read_time = Duration::from_secs(1) / max_rate as u32;
                    assert!(period > read_time, "Period must be greater than read time");
                    self.scheduler = Some(Scheduler {
                        timer: Timer::after(period),
                        period,
                    });
                    self.next().await;
                }
                Message::Stop => {
                    info!("Stopping measurement");
                    self.scheduler = None
                }
            }
        }
        /// Run the next scheduled action.
        async fn next(&mut self) {
            let Some(scheduler) = self.scheduler.take() else {
                return; // no scheduled action
            };
            let now = Instant::now();
            let period = scheduler.period;
            self.read_measurement().await;
            self.scheduler = Some(Scheduler {
                timer: Timer::after(period - now.elapsed()),
                period,
            });
        }

        /// Read the temperature and humidity from the sensor
        async fn read_measurement(&mut self) {
            match (self.device.accel_norm(), self.device.gyro_norm()) {
                (Ok(accel), Ok(gyro)) => {
                    let inclination = self.gimbal.as_mut().map(|g| g.read(gyro, accel));
                    log::info!("Gyro: {:?}", gyro);
                    log::info!("Accel: {:?}", accel);
                    log::info!("Inclination: {:?}", inclination);
                }
                _ => {
                    log::error!("Failed to read measurement");
                }
            }
        }
    }

    #[embassy_executor::task]
    /// The actor's task, to be spawned by the actor's context.
    pub(super) async fn actor_task(context: &'static ActorContext<Actor>, actor: Actor) {
        context.mount(actor).await;
    }
}
