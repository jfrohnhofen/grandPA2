//! grandPA2 MIDI Protocol Definitions & SysEx Decoders
//!
//! Provides clean mappings between hardware events (Matrix, Encoders, Faders)
//! and multi-channel USB MIDI messages sent to/received from the Host Application.

pub const SYSEX_MANUFACTURER_ID: u8 = 0x7D; // Educational/Prototype SysEx ID

/// MIDI Channels (0-indexed in code: 0 = Channel 1, 1 = Channel 2, 2 = Channel 3)
pub const CHAN_BUTTONS: u8 = 0; // Channel 1: Matrix Buttons (Notes 0..120)
pub const CHAN_ENCODERS: u8 = 1; // Channel 2: Encoders (CC 0..3) & Push Buttons (Notes 0..3)
pub const CHAN_FADERS: u8 = 2; // Channel 3: Exec Faders 1..4 (CC 0..3) & Grand Master (CC 4)

/// Number of physical RGB LEDs driven by the MCU
pub const NUM_LEDS: usize = 94;

/// Events originating from physical controls on the board
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoardEvent {
    MatrixButton { button_id: u8, pressed: bool },
    EncoderButton { index: u8, pressed: bool },
    EncoderTurn { index: u8, cw: bool },
    FaderMove { index: u8, value: u8 },
}

impl BoardEvent {
    /// Convert event into 4-byte USB MIDI packet format: [header, status, note/cc, velocity/val]
    pub fn to_usb_midi_packet(self) -> [u8; 4] {
        match self {
            BoardEvent::MatrixButton { button_id, pressed } => {
                if pressed {
                    // Code 0x09 (Note On), Channel 1
                    [0x09, 0x90 | CHAN_BUTTONS, button_id & 0x7F, 127]
                } else {
                    // Code 0x08 (Note Off), Channel 1
                    [0x08, 0x80 | CHAN_BUTTONS, button_id & 0x7F, 0]
                }
            }
            BoardEvent::EncoderButton { index, pressed } => {
                let note = index & 0x03;
                if pressed {
                    // Code 0x09 (Note On), Channel 2
                    [0x09, 0x90 | CHAN_ENCODERS, note, 127]
                } else {
                    // Code 0x08 (Note Off), Channel 2
                    [0x08, 0x80 | CHAN_ENCODERS, note, 0]
                }
            }
            BoardEvent::EncoderTurn { index, cw } => {
                let cc = index & 0x03;
                let val = if cw { 1 } else { 127 }; // Relative 2's complement: +1 (CW) / 127 (-1 CCW)
                                                    // Code 0x0B (Control Change), Channel 2
                [0x0B, 0xB0 | CHAN_ENCODERS, cc, val]
            }
            BoardEvent::FaderMove { index, value } => {
                let cc = index & 0x07; // CC 0..3 for Exec Faders 1..4, CC 4 for Grand Master
                                       // Code 0x0B (Control Change), Channel 3
                [0x0B, 0xB0 | CHAN_FADERS, cc, value & 0x7F]
            }
        }
    }
}

/// Incoming SysEx Host Commands (from PC Host Application)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostCommand {
    /// Set 24-bit RGB colors for all 94 LEDs (282 bytes of 7-bit RGB data: R0, G0, B0...)
    SetAllRgbLeds { colors: [(u8, u8, u8); NUM_LEDS] },
    /// Update 1BPP Pixel Chunk for display (display_id: 0..3, chunk_id: 0..3, 256 decoded pixel bytes)
    SetDisplayPixelChunk { display_id: u8, chunk_id: u8, pixels: [u8; 256] },
}
