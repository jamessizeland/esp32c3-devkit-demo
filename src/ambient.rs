//! A simple actor to read from the Ambient Sensor.
//!
//! This actor can be used to read from the Ambient Sensor.
//! at a set rate or on demand.

use actor_private::*;
use ector::ActorContext;
use embassy_executor::Spawner;
use log::info;
use shtcx::PowerMode;
use {
    core::future::pending,
    embassy_executor::SpawnError,
    embassy_futures::select::{Either, select},
    embassy_time::{Duration, Timer},
};

use crate::{ActorInbox, ble::GattServer, bsp::I2cBus};

/// The actor's message type, communicating the finite states of the actor.
/// This is made available to other actors to interact with this one.
pub enum Message {
    /// Set the power mode of the sensor
    SetPowerMode(PowerMode),
    /// Read the temperature and humidity from the sensor at a set period
    Start(Duration),
    /// Stop reading the temperature and humidity from the sensor
    Stop,
}

/// The actor's configuration, to be shared with other actors to initialize this actor.
pub struct Config {
    pub i2c_bus: &'static I2cBus<'static>,
    pub ble: Option<&'static GattServer<'static>>,
}

/// Create a new actor with a spawner and a configuration.
/// This pattern could be made into a macro to simplify the actor creation.
pub fn spawn_actor(spawner: Spawner, config: Config) -> Result<ActorInbox<Message>, SpawnError> {
    static CONTEXT: ActorContext<Actor> = ActorContext::new();
    let inbox = CONTEXT.address();
    spawner.spawn(actor_task(&CONTEXT, Actor::new(spawner, config, inbox)))?;
    Ok(inbox)
}

mod actor_private {

    use ector::{DynamicAddress, Inbox};
    use embassy_embedded_hal::shared_bus::blocking::i2c::I2cDevice;
    use embassy_time::Instant;
    use shtcx::ShtC3;

    use crate::bsp::I2cBusDevice;

    use super::*;
    /// A scheduler to run a sequence of actions.
    struct Scheduler {
        /// The timer to schedule the next action
        timer: Timer,
        /// The period between actions
        period: Duration,
        /// The time it takes to read from the sensor in the current power mode
        read_time: Duration,
    }

    /// The actor's private data, not to be shared with other actors.
    /// This is where the actor's state is stored.
    pub(super) struct Actor {
        /// A timer to schedule the next message
        scheduler: Option<Scheduler>,
        /// The Onboard temperature and humidity sensor
        device: ShtC3<I2cBusDevice<'static>>,
        /// The current power mode of the sensor
        power_mode: PowerMode,
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
            Self {
                scheduler: None,
                device: shtcx::shtc3(i2c),
                power_mode: PowerMode::LowPower,
            }
        }
        /// The message handler
        async fn act(&mut self, msg: Message) {
            match msg {
                Message::SetPowerMode(power_mode) => {
                    self.power_mode = power_mode;
                    info!("Power mode set to {:?}", power_mode);
                }
                Message::Start(period) => {
                    let read_time = {
                        let val = shtcx::max_measurement_duration(&self.device, self.power_mode);
                        Duration::from_millis(val.into())
                    };
                    info!("Read time: {:?} seconds", read_time.as_secs());
                    info!("Starting measurement every {:?} seconds", period.as_secs());
                    assert!(period > read_time, "Period must be greater than read time");
                    self.scheduler = Some(Scheduler {
                        timer: Timer::after(period),
                        read_time,
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
            let read_time = scheduler.read_time;
            self.read_measurement(read_time).await;
            self.scheduler = Some(Scheduler {
                timer: Timer::after(period - now.elapsed()),
                period,
                read_time,
            });
        }

        /// Read the temperature and humidity from the sensor
        async fn read_measurement(&mut self, read_time: Duration) {
            if let Err(err) = self.device.start_measurement(self.power_mode) {
                log::error!("Failed to start measurement: {:?}", err);
                return;
            };
            Timer::after(read_time).await;
            match self.device.get_measurement_result() {
                Ok(measurement) => {
                    log::info!(
                        "Temperature: {}°C, Humidity: {}%",
                        measurement.temperature.as_degrees_celsius(),
                        measurement.humidity.as_percent()
                    );
                }
                Err(err) => {
                    log::error!("Failed to read measurement: {:?}", err);
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
