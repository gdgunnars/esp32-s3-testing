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

    println!("Start NeoPixel rainbow!");
    let mut hue = unsafe { esp_random() } as u8;
    loop {
        let rainbow = rainbow_flow(hue, 5);
        ws2812.write(rainbow).unwrap();

        hue = hue.wrapping_add(4);

        FreeRtos::delay_ms(50);
    }
}

fn rainbow_flow(starting_hue: u8, pixel_count: usize) -> impl Iterator<Item = RGB8> {
    let mut hue = starting_hue;

    let pixels = std::iter::repeat_with(move || {
        let current_hue = hue;

        hue = hue.wrapping_add(32); // Increment hue by 32 for the next decided color (0-255)

        hsv2rgb(Hsv {
            hue: current_hue,
            sat: 255,
            val: 8,
        })
    })
    .take(pixel_count);

    return pixels;
}
