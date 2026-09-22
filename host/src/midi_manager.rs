use midir::{MidiInput, MidiOutput, MidiOutputConnection};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::sleep;

use crate::keymap::NUM_LEDS;

#[derive(Debug, Clone)]
pub enum HardwareInputEvent {
    /// Button event (button_id 0..93, pressed)
    Button { button_id: usize, pressed: bool },
    /// Encoder turn (encoder_id 0..3, delta +1 / -1)
    EncoderTurn { encoder_id: usize, delta: i32 },
    /// Encoder push button toggle (encoder_id 0..3, pressed)
    EncoderButton { encoder_id: usize, pressed: bool },
    /// Exec Fader move (fader_id 0..3: exec 1..4, fader_id 4: grand master, level 0..127)
    FaderMove { fader_id: usize, level: u8 },
    /// Hardware device was connected or reconnected
    DeviceConnected,
}

pub struct MidiManager;

impl MidiManager {
    /// Start the background auto-reconnecting USB MIDI manager.
    /// Returns (MPSC Receiver for hardware inputs, Shared handle for outgoing SysEx transfers).
    pub fn start(input_tx: mpsc::Sender<HardwareInputEvent>) -> Arc<Mutex<Option<MidiOutputConnection>>> {
        let midi_out_handle = Arc::new(Mutex::new(None::<MidiOutputConnection>));
        let handle_clone = Arc::clone(&midi_out_handle);

        tokio::spawn(async move {
            loop {
                // Throttle connection attempts while disconnected
                sleep(Duration::from_secs(1)).await;

                let Ok(midi_in) = MidiInput::new("grandPA2 Host In") else { continue };
                let Ok(midi_out) = MidiOutput::new("grandPA2 Host Out") else { continue };

                let Some(in_port) = midi_in
                    .ports()
                    .into_iter()
                    .find(|p| midi_in.port_name(p).map(|n| n.contains("grandPA2")).unwrap_or(false))
                else {
                    continue;
                };

                let Some(out_port) = midi_out
                    .ports()
                    .into_iter()
                    .find(|p| midi_out.port_name(p).map(|n| n.contains("grandPA2")).unwrap_or(false))
                else {
                    continue;
                };

                println!("[USB MIDI] Found grandPA2 hardware. Connecting...");

                let conn_out = match midi_out.connect(&out_port, "grandPA2-out") {
                    Ok(c) => c,
                    Err(e) => {
                        println!("[USB MIDI] MIDI output connection failed: {}", e);
                        continue;
                    }
                };

                println!("[USB MIDI] Connected to grandPA2 hardware");
                let _ = input_tx.send(HardwareInputEvent::DeviceConnected).await;

                {
                    let mut handle = handle_clone.lock().unwrap();
                    *handle = Some(conn_out);
                }

                let tx = input_tx.clone();
                let conn_in_res = midi_in.connect(
                    &in_port,
                    "grandPA2-in",
                    move |_, message, _| {
                        if message.len() < 3 {
                            return;
                        }

                        let status = message[0];
                        let cmd = status & 0xF0;
                        let chan = status & 0x0F;
                        let d1 = message[1];
                        let d2 = message[2];

                        match (chan, cmd) {
                            // Channel 1: Buttons
                            (0, 0x90 | 0x80) => {
                                let pressed = cmd == 0x90 && d2 > 0;
                                let button_id = d1 as usize;
                                if button_id < NUM_LEDS {
                                    let _ = tx.blocking_send(HardwareInputEvent::Button { button_id, pressed });
                                }
                            }
                            // Channel 2: Encoders & Push Buttons
                            (1, 0xB0) => {
                                let encoder_id = (d1 & 0x03) as usize;
                                let delta = if d2 < 64 { d2 as i32 } else { (d2 as i32) - 128 };
                                let _ = tx.blocking_send(HardwareInputEvent::EncoderTurn { encoder_id, delta });
                            }
                            (1, 0x90 | 0x80) => {
                                let encoder_id = (d1 & 0x03) as usize;
                                let pressed = cmd == 0x90 && d2 > 0;
                                let _ = tx.blocking_send(HardwareInputEvent::EncoderButton { encoder_id, pressed });
                            }
                            // Channel 3: Faders
                            (2, 0xB0) => {
                                let fader_id = (d1 & 0x07) as usize;
                                if fader_id < 5 {
                                    let _ = tx.blocking_send(HardwareInputEvent::FaderMove { fader_id, level: d2 });
                                }
                            }
                            _ => (),
                        };
                    },
                    (),
                );

                if let Err(e) = conn_in_res {
                    println!("[USB MIDI] MIDI input connection failed: {}", e);
                    if let Ok(mut handle) = handle_clone.lock() {
                        *handle = None;
                    }
                    continue;
                }

                // Stay in active monitoring loop until hardware is disconnected
                loop {
                    sleep(Duration::from_millis(500)).await;

                    // Check if grandPA2 port is still present in system
                    let still_connected = match MidiInput::new("check-in") {
                        Ok(check_in) => check_in
                            .ports()
                            .into_iter()
                            .any(|p| check_in.port_name(&p).map(|n| n.contains("grandPA2")).unwrap_or(false)),
                        Err(_) => false,
                    };

                    let handle_valid = handle_clone.lock().map(|h| h.is_some()).unwrap_or(false);

                    if !still_connected || !handle_valid {
                        println!("[USB MIDI] grandPA2 disconnected. Resuming auto-scan...");
                        break;
                    }
                }

                // Reset handle on disconnect
                if let Ok(mut handle) = handle_clone.lock() {
                    *handle = None;
                }
            }
        });

        midi_out_handle
    }
}
