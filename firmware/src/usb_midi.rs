//! USB MIDI Driver & Event Router for grandPA2
//!
//! Parses incoming USB MIDI SysEx streams (CIN 0x4..0x7) for RGB LEDs & OLED 1BPP Pixel Chunks,
//! and emits outgoing 4-byte USB MIDI packets (Notes & CCs).

use crate::protocol::{BoardEvent, HostCommand, NUM_LEDS, SYSEX_MANUFACTURER_ID};
use stm32f4xx_hal::otg_fs::UsbBusType;
use usb_device::prelude::*;
use usbd_midi::UsbMidiClass;

pub struct UsbMidiManager<'a> {
    usb_dev: UsbDevice<'a, UsbBusType>,
    midi_class: UsbMidiClass<'a, UsbBusType>,
    sysex_buf: [u8; 512],
    sysex_idx: usize,
    in_sysex: bool,
}

impl<'a> UsbMidiManager<'a> {
    pub fn new(usb_dev: UsbDevice<'a, UsbBusType>, midi_class: UsbMidiClass<'a, UsbBusType>) -> Self {
        Self { usb_dev, midi_class, sysex_buf: [0; 512], sysex_idx: 0, in_sysex: false }
    }

    /// Send a BoardEvent to PC as a 4-byte USB MIDI packet
    pub fn send_event(&mut self, event: BoardEvent) {
        let packet = event.to_usb_midi_packet();
        let _ = self.midi_class.send_bytes(packet);
    }

    /// Poll USB stack and process incoming commands from PC
    pub fn poll<F>(&mut self, mut on_command: F)
    where
        F: FnMut(HostCommand),
    {
        if !self.usb_dev.poll(&mut [&mut self.midi_class]) {
            return;
        }

        let mut buf = [0u8; 128];
        if let Ok(count) = self.midi_class.read(&mut buf) {
            let mut i = 0;
            while i + 3 < count {
                let cin = buf[i] & 0x0F;
                let b0 = buf[i + 1];
                let b1 = buf[i + 2];
                let b2 = buf[i + 3];

                match cin {
                    0x4 => self.handle_sysex_bytes(&[b0, b1, b2], &mut on_command),
                    0x5 => self.handle_sysex_bytes(&[b0], &mut on_command),
                    0x6 => self.handle_sysex_bytes(&[b0, b1], &mut on_command),
                    0x7 => self.handle_sysex_bytes(&[b0, b1, b2], &mut on_command),
                    _ => {}
                }

                i += 4;
            }
        }
    }

    fn handle_sysex_bytes<F>(&mut self, bytes: &[u8], on_command: &mut F)
    where
        F: FnMut(HostCommand),
    {
        for &byte in bytes {
            if byte == 0xF0 {
                self.in_sysex = true;
                self.sysex_idx = 0;
            } else if byte == 0xF7 {
                if self.in_sysex {
                    self.in_sysex = false;
                    self.process_sysex(on_command);
                }
            } else if self.in_sysex {
                if self.sysex_idx < self.sysex_buf.len() {
                    self.sysex_buf[self.sysex_idx] = byte;
                    self.sysex_idx += 1;
                }
            }
        }
    }

    fn process_sysex<F>(&mut self, on_command: &mut F)
    where
        F: FnMut(HostCommand),
    {
        // Minimum header: [SYSEX_MANUFACTURER_ID, CMD_ID]
        if self.sysex_idx < 2 || self.sysex_buf[0] != SYSEX_MANUFACTURER_ID {
            return;
        }

        let cmd_id = self.sysex_buf[1];
        let payload = &self.sysex_buf[2..self.sysex_idx];

        match cmd_id {
            0x10 => {
                // RGB LEDs update: 94 LEDs * 3 bytes (R, G, B) = 282 bytes
                if payload.len() >= NUM_LEDS * 3 {
                    let mut colors = [(0u8, 0u8, 0u8); NUM_LEDS];
                    for i in 0..NUM_LEDS {
                        let r = payload[i * 3] & 0x7F;
                        let g = payload[i * 3 + 1] & 0x7F;
                        let b = payload[i * 3 + 2] & 0x7F;
                        colors[i] = (r, g, b);
                    }
                    on_command(HostCommand::SetAllRgbLeds { colors });
                }
            }
            0x20..=0x2F => {
                // OLED 1BPP Pixel Chunk: DisplayID = (cmd - 0x20)/4, ChunkID = (cmd - 0x20)%4
                let rel = cmd_id - 0x20;
                let display_id = rel / 4;
                let chunk_id = rel % 4;

                let mut pixels = [0u8; 256];
                unpack_7to8(payload, &mut pixels);

                on_command(HostCommand::SetDisplayPixelChunk { display_id, chunk_id, pixels });
            }
            _ => {}
        }
    }
}

/// Unpack 7-bit MIDI payload bytes into 8-bit raw bytes with bit buffer mask
fn unpack_7to8(input: &[u8], output: &mut [u8]) {
    let mut bit_buf: u32 = 0;
    let mut bits_in_buf: u8 = 0;
    let mut out_idx = 0;

    for &byte in input {
        bit_buf = (bit_buf << 7) | ((byte & 0x7F) as u32);
        bits_in_buf += 7;

        while bits_in_buf >= 8 {
            bits_in_buf -= 8;
            if out_idx < output.len() {
                output[out_idx] = ((bit_buf >> bits_in_buf) & 0xFF) as u8;
                out_idx += 1;
            }
            if bits_in_buf > 0 {
                bit_buf &= (1 << bits_in_buf) - 1;
            } else {
                bit_buf = 0;
            }
        }
    }
}
