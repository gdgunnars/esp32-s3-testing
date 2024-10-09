mod http_client;
mod led_controller;
mod led_state;
mod wifi;

use anyhow::{Error, Result};
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    hal::{delay::FreeRtos, prelude::Peripherals},
    sys::nvs_flash_init,
    timer::EspTaskTimerService,
};
use led_controller::LedController;
use led_state::{AtomicLedState, LedState};
use std::thread;
use std::{
    sync::{atomic::Ordering, OnceLock},
    time::Duration,
};

// LED state shared between modules
static LED_STATE: OnceLock<AtomicLedState> = OnceLock::new();

const SSID: &str = env!("WIFI_SSID");
const PASSWORD: &str = env!("WIFI_PASS");

fn main() -> Result<(), Error> {
    // Necessary call to link patches for the runtime
    esp_idf_svc::sys::link_patches();
    unsafe {
        nvs_flash_init();
    }

    // Setup
    esp_idf_svc::log::EspLogger::initialize_default();
    LED_STATE.get_or_init(|| AtomicLedState::new(LedState::INIT));
    let peripherals = Peripherals::take().unwrap();

    // LED controller setup
    let led_pin_strip = peripherals.pins.gpio10; // Pin for LED strip
    let channel2 = peripherals.rmt.channel2; // RMT channel for LED strip
    thread::spawn(move || {
        let mut led_controller = LedController::new(led_pin_strip, channel2, &LED_STATE)
            .expect("Failed to initialize LED controller");
        led_controller.run().expect("Failed to run LED loop");
    });

    // Connect to the Wi-Fi network
    if SSID.contains("CHANGE_ME") || PASSWORD.contains("CHANGE_ME") {
        panic!("Please set your Wi-Fi credentials in config.toml");
    }
    // unsafe { nvs_flash_init() };
    let sysloop = EspSystemEventLoop::take()?;
    let _wifi = wifi::wifi(SSID, PASSWORD, peripherals.modem, sysloop)?;

    LED_STATE
        .get()
        .unwrap()
        .store(LedState::CLEAR, Ordering::SeqCst);

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

    // Main loop can now handle other tasks
    log::info!("Starting main thread loop");
    loop {
        let url =
            "https://cdn.jsdelivr.net/npm/@fawazahmed0/currency-api@latest/v1/currencies/dkk.json";
        match http_client::get(url) {
            Ok(_) => {
                log::info!("HttpClientSuccess!");
                // ws2812.write(solid_color(colors::AQUA, 50)).unwrap();
            }
            Err(e) => {
                log::error!("-> Caught the HttpClient failure: {:?}", e);
                // Setting onboard LED to red as we couldn't fetch data from the URL;
                // ws2812.write(solid_color(colors::RED, 50)).unwrap();
            }
        };

        FreeRtos::delay_ms(30_000);
    }
}
