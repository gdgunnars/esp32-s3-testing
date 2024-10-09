use crate::led_state::{AtomicLedState, LedState};
use anyhow::Result;
use esp_idf_svc::hal::delay::FreeRtos;
use esp_idf_svc::hal::gpio::OutputPin;
use esp_idf_svc::hal::peripheral::Peripheral;
use esp_idf_svc::hal::rmt::RmtChannel;
use smart_leds::{
    brightness, colors,
    hsv::{hsv2rgb, Hsv},
    SmartLedsWrite, RGB8,
};
use std::{f32::consts::PI, sync::atomic::Ordering};
use std::{marker::PhantomData, sync::OnceLock};
use ws2812_esp32_rmt_driver::Ws2812Esp32Rmt;

const LED_COUNT: usize = 5;

pub struct LedController<'d, C, P>
where
    C: RmtChannel,
    P: OutputPin,
{
    ws2812: Ws2812Esp32Rmt<'d>,
    phantom: PhantomData<(C, P)>,
    hue: u8,
    brightness: u8,
    led_state: &'d OnceLock<AtomicLedState>, // Add reference to LED_STATE
}

impl<'d, C, P> LedController<'d, C, P>
where
    C: RmtChannel,
    P: OutputPin,
{
    pub fn new(
        led_pin_strip: impl Peripheral<P = P> + 'd,
        channel: impl Peripheral<P = C> + 'd,
        led_state: &'d OnceLock<AtomicLedState>, // Pass in the LED_STATE
    ) -> Result<Self, anyhow::Error> {
        let ws2812 = Ws2812Esp32Rmt::new(channel, led_pin_strip)?;
        Ok(Self {
            ws2812,
            phantom: Default::default(),
            hue: 0,
            brightness: 50,
            led_state, // Store the LED_STATE reference
        })
    }

    pub fn run(&mut self) -> Result<(), anyhow::Error> {
        log::info!("LED loop started");

        loop {
            // Load the atomic value and convert it to the enum
            let state = self.led_state.get().unwrap().load(Ordering::Relaxed);
            // Get the appropriate LED colors based on the current state
            let led_colors = self.get_led_strip_colors(state);

            // Write the LED colors to the strip
            if let Err(e) = self.ws2812.write(led_colors) {
                log::error!("Failed to update LED strip: {:?}", e);
            }

            FreeRtos::delay_ms(50); // Delay to limit the loop frequency
        }
    }

    fn get_led_strip_colors(&mut self, state: LedState) -> Box<dyn Iterator<Item = RGB8>> {
        match state {
            LedState::INIT => {
                self.brightness = self.brightness.wrapping_add(4);
                Box::new(breathing_effect(colors::WHITE, self.brightness))
            } // Breathing effect
            LedState::PARTY => {
                self.hue = self.hue.wrapping_add(4);
                Box::new(rainbow_flow(self.hue))
            }
            LedState::CLEAR => Box::new(solid_color(colors::WHITE, 50)),
            LedState::ERROR => Box::new(solid_color(colors::RED, 50)),
            LedState::WARNING => Box::new(solid_color(colors::YELLOW, 50)),
        }
    }
}

// Helper functions for LED colors
fn rainbow_flow(starting_hue: u8) -> impl Iterator<Item = RGB8> {
    let mut hue = starting_hue;

    std::iter::repeat_with(move || {
        let current_hue = hue;

        hue = hue.wrapping_add(32); // Increment hue by 32 for the next decided color (0-255)

        hsv2rgb(Hsv {
            hue: current_hue,
            sat: 255,
            val: 8,
        })
    })
    .take(LED_COUNT)
}

fn solid_color(color: RGB8, level: u8) -> impl Iterator<Item = RGB8> {
    brightness(std::iter::repeat(color).take(LED_COUNT), level)
}

// Breathing effect function
fn breathing_effect(color: RGB8, step: u8) -> impl Iterator<Item = RGB8> {
    let min_brightness = 25.0;
    let max_brightness = 255.0;
    let brightness_range = max_brightness - min_brightness;

    // Scale step to cycle over 0 to 2 * PI smoothly
    let scaled_step = step as f32 * (2.0 * PI / 255.0);

    // Compute brightness as a smooth sinusoidal wave, scaled to [min_brightness, max_brightness]
    let brightness_level =
        ((scaled_step.sin() + 1.0) / 2.0 * brightness_range + min_brightness) as u8;

    // Apply the brightness level to the color
    solid_color(color, brightness_level)
}
