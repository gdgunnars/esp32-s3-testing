mod http_client;
mod led_controller;
mod led_state;
mod wifi;

use anyhow::{Error, Result};
use esp_idf_svc::{
    eventloop::EspSystemEventLoop, hal::prelude::Peripherals, sys::nvs_flash_init,
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
    LED_STATE.get_or_init(|| AtomicLedState::new(LedState::Init));
    let peripherals = Peripherals::take().unwrap();

    // LED controller setup
    let led_pin_strip = peripherals.pins.gpio10; // Pin for LED strip
    let channel2 = peripherals.rmt.channel2; // RMT channel for LED strip
    let led_thread = thread::spawn(move || {
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
        .store(LedState::Clear, Ordering::SeqCst);

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

    // API call
    let api_timer_service = EspTaskTimerService::new()?;
    let api_callback_timer = {
        api_timer_service.timer(move || {
            let url = "https://cdn.jsdelivr.net/npm/@fawazahmed0/currency-api@latest/v1/currencies/dkk.json";
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
        })?
    };

    api_callback_timer.every(Duration::from_secs(10))?;

    // Main thread waits for other threads to finish (they won't in this case)
    log::info!("Starting main thread and waiting for background tasks");

    // Joining the LED thread ensures the main function doesn't exit
    led_thread.join().unwrap();

    // Optionally: add logic here if you want to wait for specific other threads or services
    Ok(())
}
