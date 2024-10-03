mod http_client;
mod led_state;
mod wifi;

use anyhow::{Error, Result};
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    hal::{delay::FreeRtos, prelude::Peripherals},
    timer::EspTaskTimerService,
};
use led_state::{AtomicLedState, LedState};
use log::{error, info};
use smart_leds::{
    brightness, colors,
    hsv::{hsv2rgb, Hsv},
    SmartLedsWrite, RGB8,
};
use std::{
    sync::{atomic::Ordering, OnceLock},
    time::Duration,
};
use ws2812_esp32_rmt_driver::Ws2812Esp32Rmt;

const LED_COUNT: usize = 5;

const SSID: &str = env!("WIFI_SSID");
const PASSWORD: &str = env!("WIFI_PASS");

// Use OnceLock to lazily initialize AtomicLedState safely in a multithreaded context
static LED_STATE: OnceLock<AtomicLedState> = OnceLock::new();

fn main() -> Result<(), Error> {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();
    // -----------------------------------------------
    // Setup

    // Lazily initialize the LED_STATE with an initial value (e.g., LedState::CLEAR)
    LED_STATE.get_or_init(|| AtomicLedState::new(LedState::CLEAR));

    let peripherals = Peripherals::take().unwrap();

    // Connect to the Wi-Fi network
    if SSID.contains("CHANGE_ME") || PASSWORD.contains("CHANGE_ME") {
        panic!("Please set your Wi-Fi credentials in config.toml");
    }
    let sysloop = EspSystemEventLoop::take()?;
    let _wifi = wifi::wifi(SSID, PASSWORD, peripherals.modem, sysloop)?;

    // Configure onboard LED
    let led_pin = peripherals.pins.gpio21;
    let channel = peripherals.rmt.channel0;
    let mut ws2812 = Ws2812Esp32Rmt::new(channel, led_pin)?;

    // -----------------------------------------------
    // Call API every 30 seconds

    let api_timer_service = EspTaskTimerService::new()?;
    let api_callback_timer = {
        api_timer_service.timer(move || {
            let url = "https://cdn.jsdelivr.net/npm/@fawazahmed0/currency-api@latest/v1/currencies/dkk.json";
            match http_client::get(url) {
                Ok(_) => {
                    info!("HttpClientSuccess!");
                    ws2812.write(solid_color(colors::AQUA, 50)).unwrap();
                }
                Err(e) => {
                    error!("-> Caught the HttpClient failure: {:?}", e);
                    // Setting onboard LED to red as we couldn't fetch data from the URL;
                    ws2812.write(solid_color(colors::RED, 50)).unwrap();
                }
            };
        })?
    };

    api_callback_timer.every(Duration::from_secs(30))?;

    // -----------------------------------------------
    // Change LED state every 10 seconds for testing

    let color_change_timer_service = EspTaskTimerService::new()?;
    let color_change_timer = {
        color_change_timer_service.timer(move || {
            LED_STATE.get().unwrap().increment(Ordering::SeqCst);
            let new_state = LED_STATE.get().unwrap().load(Ordering::SeqCst);
            log::info!("LED state changed: {:?}", new_state); // Log the new state
        })?
    };

    color_change_timer.every(Duration::from_secs(10))?;

    // -----------------------------------------------

    // Setup LED strip channel
    let led_pin_strip = peripherals.pins.gpio10;
    let channel2 = peripherals.rmt.channel2;
    let mut ws2812_strip = Ws2812Esp32Rmt::new(channel2, led_pin_strip)?;

    // Set base hue that will update if party state is active
    let mut hue: u8 = 0;
    println!("Setup finished, State matching started!");

    loop {
        // Load the atomic value and convert it to the enum
        let state = LED_STATE.get().unwrap().load(Ordering::Relaxed);

        let led_colors = get_led_strip_colors(state, &mut hue);

        if let Err(e) = ws2812_strip.write(led_colors) {
            log::error!("Failed to update LED strip: {:?}", e);
        }

        FreeRtos::delay_ms(50);
    }
    // Ok(())
}

fn get_led_strip_colors(state: LedState, hue: &mut u8) -> Box<dyn Iterator<Item = RGB8>> {
    match state {
        LedState::INIT => Box::new(solid_color(colors::WHITE, 50)),
        LedState::PARTY => {
            *hue = hue.wrapping_add(4);
            Box::new(rainbow_flow(*hue))
        }
        LedState::CLEAR => Box::new(solid_color(colors::WHITE, 50)),
        LedState::ERROR => Box::new(solid_color(colors::RED, 50)),
        LedState::WARNING => Box::new(solid_color(colors::YELLOW, 50)),
    }
}

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
