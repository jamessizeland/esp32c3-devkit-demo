use esp_hal_smartled::LedAdapterError;
use smart_leds::{RGB8, SmartLedsWrite, brightness};

use crate::bsp;

/// Set the colour and brightness of the specified LED.
pub fn write_led(led: &mut bsp::Led, colour: RGB8, level: u8) -> Result<(), LedAdapterError> {
    led.write(brightness([colour].into_iter(), level))
}
