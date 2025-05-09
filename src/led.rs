//! A simple actor to control an LED.
//!
//! This actor can be used to control an LED with a sequence of colours.
//! The actor can be controlled by sending messages to it.
//! The actor can be created with a spawner and a configuration.

use actor_private::*;
use ector::ActorContext;
use embassy_executor::Spawner;
use esp_hal::rmt;
use esp_hal_smartled::SmartLedsAdapter;
use log::{error, info};
use smart_leds::{RGB8, SmartLedsWrite, brightness, colors::BLACK};
use {
    core::future::pending,
    embassy_executor::SpawnError,
    embassy_futures::select::{Either, select},
    embassy_time::{Duration, Timer},
};

use crate::ActorInbox;

pub type Led = SmartLedsAdapter<rmt::Channel<esp_hal::Blocking, 0>, 25>;

/// Set the colour and brightness of the specified LED.
pub fn write(led: &mut Led, colour: RGB8, level: u8) {
    if let Err(err) = led.write(brightness([colour].into_iter(), level)) {
        error!("Failed to write to LED: {:?}", err);
    };
}

/// The actor's repeat mode.
#[derive(Clone, Copy)]
pub enum Repeat {
    /// Run the sequence once
    Once,
    /// Run the sequence a fixed number of times
    N(u8),
    /// Run the sequence forever
    Forever,
}

/// The actor's message type, communicating the finite states of the actor.
/// This is made available to other actors to interact with this one.
pub enum Message {
    /// Set the colour of the LED
    SetColour(RGB8),
    /// Set the brightness of the LED
    SetBrightness(u8),
    /// Turn the LED off
    Off,
    /// Turn the LED on
    On,
    /// Set the LED to a sequence of colours
    SetSequence((&'static [RGB8], Duration, Repeat)),
}

/// The actor's configuration, to be shared with other actors to initialize this actor.
pub struct Config {
    pub led: Led,
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

    use super::*;
    /// A scheduler to run a sequence of actions.
    struct Scheduler {
        /// The timer to schedule the next action
        timer: Timer,
        /// The period between actions
        period: Duration,
        /// The current sequence of colours
        sequence: &'static [RGB8],
        /// The current index in the sequence
        index: usize,
        /// The current repeat mode
        repeat: Repeat,
    }

    /// The actor's private data, not to be shared with other actors.
    /// This is where the actor's state is stored.
    pub(super) struct Actor {
        /// A timer to schedule the next message
        scheduler: Option<Scheduler>,
        /// The LED to control
        led: Led,
        /// The current colour of the LED
        colour: RGB8,
        /// The current brightness of the LED
        /// This is a percentage from 0 to 100
        brightness: u8,
    }

    impl ector::Actor for Actor {
        type Message = Message;

        /// Actor pattern for either handling new incoming messages or running a scheduled action.
        async fn on_mount<M>(&mut self, _: DynamicAddress<Message>, mut inbox: M) -> !
        where
            M: Inbox<Self::Message>,
        {
            info!("LED Task started!");
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
            // Opportunity to do any setup before mounting the actor
            // this could include spawning child actors or setting up resources
            // we have access to our own inbox here to send down to child actors.
            Self {
                led: config.led,
                scheduler: None,
                colour: RGB8 { r: 0, g: 0, b: 0 },
                brightness: 50,
            }
        }
        /// The message handler
        async fn act(&mut self, msg: Message) {
            self.scheduler = None; // cancel any scheduled actions
            match msg {
                Message::SetColour(colour) => {
                    self.colour = colour;
                    write(&mut self.led, colour, self.brightness)
                }
                Message::SetBrightness(level) => {
                    self.brightness = level;
                    write(&mut self.led, self.colour, level)
                }
                Message::Off => write(&mut self.led, BLACK, 0),
                Message::On => write(&mut self.led, self.colour, self.brightness),
                Message::SetSequence((sequence, period, repeat)) => {
                    self.scheduler = Some(Scheduler {
                        timer: Timer::after(period),
                        period,
                        sequence,
                        index: 0,
                        repeat,
                    });
                }
            }
        }
        /// Run the next scheduled action.
        async fn next(&mut self) {
            let Some(scheduler) = self.scheduler.as_mut() else {
                return; // no scheduled action
            };
            scheduler.timer = Timer::after(scheduler.period);
            // run the next action in the sequence.
            match scheduler.sequence.get(scheduler.index) {
                Some(&colour) => {
                    write(&mut self.led, colour, self.brightness);
                    scheduler.index += 1;
                }
                None => {
                    // if we've reached the end of the sequence, handle the repeat mode.
                    match scheduler.repeat {
                        Repeat::Once => self.scheduler = None,
                        Repeat::N(0) => self.scheduler = None,
                        Repeat::N(n) => scheduler.repeat = Repeat::N(n - 1),
                        Repeat::Forever => scheduler.index = 0,
                    }
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
