mod http_client;
mod wifi;

use anyhow::{Error, Result};
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    hal::{delay::FreeRtos, prelude::Peripherals},
    sys::esp_random,
    timer::EspTaskTimerService,
};
use log::{error, info};
use smart_leds::{
    hsv::{hsv2rgb, Hsv},
    SmartLedsWrite, RGB8,
};
use std::{
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc,
    },
    time::Duration,
};
use ws2812_esp32_rmt_driver::Ws2812Esp32Rmt;

const COLOR_FAILURE: [RGB8; 1] = [RGB8 { g: 32, r: 00, b: 0 }];
const COLOR_RUNNING: [RGB8; 1] = [RGB8 { g: 0, r: 32, b: 32 }];

const SSID: &str = env!("WIFI_SSID");
const PASSWORD: &str = env!("WIFI_PASS");

fn main() -> Result<(), Error> {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();
    // -----------------------------------------------
    // Setup
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
    // Test url (open currency exchange rate)
    let url =
        "https://cdn.jsdelivr.net/npm/@fawazahmed0/currency-api@latest/v1/currencies/dkk.json";
    match http_client::get(url) {
        Ok(_) => {
            info!("HttpClientSuccess!");
            ws2812.write(COLOR_RUNNING)?;
        }
        Err(e) => {
            error!("################ Caught the HttpClient failure: {:?}", e);
            // Setting onboard LED to red as we couldn't fetch data from the URL;
            ws2812.write(COLOR_FAILURE)?;
        }
    };

    // -----------------------------------------------
    // Bump the counter every 10 seconds;
    let counter = Arc::new(AtomicU32::new(0));

    let timer_service = EspTaskTimerService::new()?;
    let callback_timer = {
        let counter = counter.clone();
        timer_service.timer(move || {
            let current = counter.fetch_add(1, Ordering::SeqCst);

            info!("Callback timer reports tick: {}", current);
        })?
    };

    callback_timer.every(Duration::from_secs(10))?;

    // -----------------------------------------------

    // Setup LED strip channel
    let led_pin_strip = peripherals.pins.gpio10;
    let channel2 = peripherals.rmt.channel2;
    let mut ws2812_strip = Ws2812Esp32Rmt::new(channel2, led_pin_strip)?;

    // Set base hue that will update each iteration
    let mut hue = unsafe { esp_random() } as u8;
    println!("Start NeoPixel rainbow!");
    loop {
        let rainbow = rainbow_flow(hue, 5);

        ws2812_strip.write(rainbow)?;

        hue = hue.wrapping_add(4);

        FreeRtos::delay_ms(50);
    }
}

fn rainbow_flow(starting_hue: u8, pixel_count: usize) -> impl Iterator<Item = RGB8> {
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
    .take(pixel_count)
}
