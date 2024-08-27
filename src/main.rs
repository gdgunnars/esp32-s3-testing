use esp_idf_svc::{
    hal::{delay::FreeRtos, prelude::Peripherals},
    sys::esp_random,
};
use smart_leds::{
    hsv::{hsv2rgb, Hsv},
    SmartLedsWrite, RGB8,
};
use ws2812_esp32_rmt_driver::Ws2812Esp32Rmt;

fn main() {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();
    // -----------------------------------------------
    let peripherals = Peripherals::take().unwrap();

    // Configure onboard LED
    let led_pin = peripherals.pins.gpio21;
    let channel = peripherals.rmt.channel0;
    let mut ws2812 = Ws2812Esp32Rmt::new(channel, led_pin).unwrap();

    // -----------------------------------------------

    // Setup LED strip channel
    let led_pin_strip = peripherals.pins.gpio10;
    let channel2 = peripherals.rmt.channel2;
    let mut ws2812_strip = Ws2812Esp32Rmt::new(channel2, led_pin_strip).unwrap();

    // Set base hue that will update each iteration
    let mut hue = unsafe { esp_random() } as u8;
    println!("Start NeoPixel rainbow!");
    loop {
        let rainbow = rainbow_flow(hue, 1);
        let rainbow_strip = rainbow_flow(hue, 5);

        ws2812.write(rainbow).unwrap();
        ws2812_strip.write(rainbow_strip).unwrap();

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
