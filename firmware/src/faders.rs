//! 5-Channel Analog Fader Module (ADC1 with EMA smoothing & noise deadband)

use crate::protocol::BoardEvent;
use core::ops::FnMut;
use stm32f4xx_hal::{
    adc::{config::SampleTime, Adc},
    gpio::{Analog, PA0, PA1, PA2, PA3, PA4},
    pac::ADC1,
};

pub struct FaderSystem {
    adc: Adc<ADC1>,
    fader0: PA0<Analog>,
    fader1: PA1<Analog>,
    fader2: PA2<Analog>,
    fader3: PA3<Analog>,
    fader4: PA4<Analog>,

    filtered_vals: [u16; 5],
    last_midi_vals: [u8; 5],
    last_sent_12b: [u16; 5],
}

impl FaderSystem {
    pub fn new(
        adc: Adc<ADC1>,
        fader0: PA0<Analog>,
        fader1: PA1<Analog>,
        fader2: PA2<Analog>,
        fader3: PA3<Analog>,
        fader4: PA4<Analog>,
    ) -> Self {
        Self {
            adc,
            fader0,
            fader1,
            fader2,
            fader3,
            fader4,
            filtered_vals: [0; 5],
            last_midi_vals: [255; 5], // 255 ensures initial value is sent on first read
            last_sent_12b: [0; 5],
        }
    }

    /// Read faders, smooth values, and trigger events when MIDI 7-bit value changes
    pub fn poll<F>(&mut self, mut on_event: F)
    where
        F: FnMut(BoardEvent),
    {
        let raw_vals: [u16; 5] = [
            self.adc.convert(&mut self.fader0, SampleTime::Cycles_480),
            self.adc.convert(&mut self.fader1, SampleTime::Cycles_480),
            self.adc.convert(&mut self.fader2, SampleTime::Cycles_480),
            self.adc.convert(&mut self.fader3, SampleTime::Cycles_480),
            self.adc.convert(&mut self.fader4, SampleTime::Cycles_480),
        ];

        for i in 0..5 {
            let raw = raw_vals[i];

            // Exponential moving average filter: EMA = alpha * raw + (1-alpha) * prev
            // Using alpha = 1/16 for smooth response
            if self.filtered_vals[i] == 0 {
                self.filtered_vals[i] = raw;
                self.last_sent_12b[i] = raw;
            } else {
                let prev = self.filtered_vals[i] as u32;
                let filtered = (prev * 15 + raw as u32) / 16;
                self.filtered_vals[i] = filtered as u16;
            }

            let filtered_12b = self.filtered_vals[i];
            let last_12b = self.last_sent_12b[i];
            let diff_12b = if filtered_12b > last_12b { filtered_12b - last_12b } else { last_12b - filtered_12b };

            let midi_val = (filtered_12b >> 5) as u8;
            let clamped_val = if midi_val > 127 { 127 } else { midi_val };

            // Apply deadband on 12-bit values (~1% = 40)
            if diff_12b >= 40 || clamped_val == 0 || clamped_val == 127 {
                self.last_sent_12b[i] = filtered_12b;
                if clamped_val != self.last_midi_vals[i] {
                    self.last_midi_vals[i] = clamped_val;
                    on_event(BoardEvent::FaderMove { index: i as u8, value: clamped_val });
                }
            }
        }
    }
}
