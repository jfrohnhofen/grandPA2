//! WS2812B 94 RGB LED Strip / Backlight Driver
//!
//! Accepts 94 RGB LEDs with 7-bit color components (0..127 scaled to 0..254 via `val << 1`).

use crate::protocol::NUM_LEDS;
use cortex_m::asm::delay;
use embedded_hal::digital::v2::OutputPin;
use smart_leds::RGB8;

pub struct LedSystem<P: OutputPin> {
    pin: P,
    led_buffer: [RGB8; NUM_LEDS],
    dirty: bool,
}

impl<P: OutputPin> LedSystem<P> {
    pub fn new(mut pin: P) -> Self {
        pin.set_low().ok();

        // Hold the line LOW for at least 300us (1ms to be safe) to ensure the
        // WS2812 LEDs reset and treat the next pulses as a new frame.
        // At 48MHz, 48,000 cycles = 1ms.
        cortex_m::asm::delay(48_000);

        Self { pin, led_buffer: [RGB8 { r: 20 << 1, g: 7 << 1, b: 0 }; NUM_LEDS], dirty: true }
    }

    /// Set all 94 LEDs from 7-bit RGB tuples (r7, g7, b7)
    pub fn set_all_rgb_7bit(&mut self, colors: &[(u8, u8, u8); NUM_LEDS]) {
        for i in 0..NUM_LEDS {
            let r = colors[i].0 << 1;
            let g = colors[i].1 << 1;
            let b = colors[i].2 << 1;
            let color = RGB8 { r, g, b };
            if self.led_buffer[i] != color {
                self.led_buffer[i] = color;
                self.dirty = true;
            }
        }
    }

    /// Flush buffer to WS2812 LEDs using 48 MHz bit-bang timing
    pub fn update(&mut self) {
        if !self.dirty {
            return;
        }
        self.dirty = false;

        cortex_m::interrupt::free(|_| {
            for c in &self.led_buffer {
                let grb = [c.g, c.r, c.b];
                for byte in grb {
                    for i in (0..8).rev() {
                        let bit = (byte >> i) & 1;
                        if bit == 1 {
                            self.pin.set_high().ok();
                            delay(8);
                            self.pin.set_low().ok();
                            delay(2);
                        } else {
                            self.pin.set_high().ok();
                            delay(2);
                            self.pin.set_low().ok();
                            delay(8);
                        }
                    }
                }
            }
            self.pin.set_low().ok();
            delay(4800); // 300 µs RESET latch pulse
        });
    }
}
