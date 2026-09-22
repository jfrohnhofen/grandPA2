mod display_renderer;
mod grandma2_client;
mod keymap;
mod midi_manager;
mod screen_capture;
use display_renderer::DisplayState;
use grandma2_client::{ButtonFunction, ExecUpdate, GrandMA2Client, GrandMA2Command, GrandMA2Event};
use keymap::{KeyFunction, MASTER_KEY_MAP};
use midi_manager::{HardwareInputEvent, MidiManager};
use std::error::Error;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

fn get_encoder_context(encoder_id: usize, state: &DisplayState) -> Option<(&'static str, &'static str, bool, bool)> {
    let fine = state.columns[encoder_id].encoder.fine_mode;
    let is_active = state.columns[encoder_id].encoder.group_active;
    match encoder_id {
        0 => Some(("Dimmer", "Dim", fine, is_active)),
        1 => Some(("Position", "Pan", fine, is_active)),
        2 => Some(("Position", "Tilt", fine, is_active)),
        3 => Some(("Focus", "Zoom", fine, is_active)),
        _ => None,
    }
}

async fn activate_preset_if_needed(preset_type: &str, is_active: bool, grandma2: &mpsc::Sender<GrandMA2Command>) {
    if !is_active {
        let _ = grandma2.send(GrandMA2Command::Command { cmd: format!("PresetType \"{}\"", preset_type) }).await;
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("=== grandPA2 Host Application ===");

    // Channels for hardware input and grandMA2 network events
    let (input_tx, mut input_rx) = mpsc::channel::<HardwareInputEvent>(1000);
    let (grandma2_tx, mut grandma2_rx) = mpsc::channel::<GrandMA2Event>(1000);
    let (led_tx, mut led_rx) = mpsc::channel::<Vec<(usize, bool)>>(1000);

    // The single source of truth for the physical displays
    let display_state = Arc::new(Mutex::new(DisplayState::new()));

    let grandma2 = GrandMA2Client::start(grandma2_tx.clone());
    let midi_handle = MidiManager::start(input_tx);

    // Screen capture driver
    screen_capture::windows_capture::start(led_tx);

    // Render driver
    let display_state_for_render = Arc::clone(&display_state);
    let midi_handle_for_render = Arc::clone(&midi_handle);
    crate::display_renderer::start(display_state_for_render, midi_handle_for_render);

    let mut last_encoder_press: [Option<std::time::Instant>; 4] = [None; 4];

    loop {
        tokio::select! {
            Some(evt) = input_rx.recv() => {
                match evt {
                    HardwareInputEvent::DeviceConnected => {
                        display_state.lock().unwrap().force_redraw = true;
                    }
                    HardwareInputEvent::Button { button_id, pressed } => {
                        let key = &MASTER_KEY_MAP[button_id];
                        match key.function {
                            KeyFunction::HardKey(code) => {
                                let _ = grandma2.send(GrandMA2Command::HardKey { code, pressed }).await;
                            }
                            KeyFunction::ExecButton(exec_num) => {
                                if exec_num >= 6 && exec_num <= 9 {
                                        let disp_idx = (exec_num - 6) as usize;
                                        let mut state = display_state.lock().unwrap();
                                        state.columns[disp_idx].button.is_pressed = pressed;
                                        state.columns[disp_idx].button.flash_time = Some(std::time::Instant::now());
                                }
                                let _ = grandma2.send(GrandMA2Command::ExecButton { exec_num, pressed }).await;
                            }
                        }
                    }
                    HardwareInputEvent::EncoderTurn { encoder_id, delta } => {
                        let context = {
                            let state = display_state.lock().unwrap();
                            get_encoder_context(encoder_id, &state)
                        };

                        if let Some((preset_type, attr, fine, is_active)) = context {
                            activate_preset_if_needed(preset_type, is_active, &grandma2).await;
                            let _ = grandma2.send(GrandMA2Command::Encoder {
                                attribute: attr.to_string(),
                                delta,
                                fine,
                            }).await;
                        }
                    }
                    HardwareInputEvent::EncoderButton { encoder_id, pressed } => {
                        if pressed {
                            let now = std::time::Instant::now();
                            let is_double_tap = if let Some(last) = last_encoder_press[encoder_id] {
                                now.duration_since(last).as_millis() < 300
                            } else {
                                false
                            };
                            last_encoder_press[encoder_id] = Some(now);
                            let context = {
                                let mut state = display_state.lock().unwrap();
                                state.columns[encoder_id].encoder.fine_mode = !state.columns[encoder_id].encoder.fine_mode;
                                get_encoder_context(encoder_id, &state)
                            };

                            if let Some((preset_type, attr, _, is_active)) = context {
                                if is_double_tap {
                                    let _ = grandma2.send(GrandMA2Command::Command {
                                        cmd: format!("Off Attribute \"{}\"", attr)
                                    }).await;
                                    last_encoder_press[encoder_id] = None;
                                } else {
                                    activate_preset_if_needed(preset_type, is_active, &grandma2).await;
                                }
                            }
                        }
                    }
                    HardwareInputEvent::FaderMove { fader_id, level } => {
                        if fader_id < 4 {
                            {
                                let mut state = display_state.lock().unwrap();
                                state.columns[fader_id].fader.value = level;
                            }
                            let _ = grandma2.send(GrandMA2Command::ExecFader {
                                exec_num: (fader_id + 1) as u8,
                                level,
                            }).await;
                        } else {
                            let _ = grandma2.send(GrandMA2Command::GrandMaster { level }).await;
                        }
                    }
                }
            }

            Some(evt) = grandma2_rx.recv() => {
                match evt {
                    GrandMA2Event::PresetTypesUpdate(encoders) => {
                        let mut state = display_state.lock().unwrap();
                        for (i, enc) in encoders.into_iter().enumerate() {
                            state.columns[i].encoder = crate::display_renderer::EncoderState {
                                value: enc.value,
                                active: enc.active,
                                programmer: enc.programmer,
                                group_active: enc.group_active,
                                fine_mode: state.columns[i].encoder.fine_mode,
                            };
                        }
                    }
                    GrandMA2Event::PlaybacksUpdate(updates) => {
                        let mut state = display_state.lock().unwrap();
                        for update in updates {
                            match update {
                                ExecUpdate::Fader(idx, fader_exec) => {
                                    if idx < 4 {
                                        state.columns[idx].fader.exec = fader_exec;
                                    }
                                }
                                ExecUpdate::Button(idx, button_exec) => {
                                    if idx >= 5 && idx <= 8 {
                                        let slot_idx = idx - 5;
                                        let led_idx = 90 + slot_idx;
                                        state.leds[led_idx] = match button_exec.function {
                                            ButtonFunction::Empty => crate::display_renderer::LedState::Off,
                                            ButtonFunction::Toggle if button_exec.active => crate::display_renderer::LedState::Active,
                                            ButtonFunction::Toggle => crate::display_renderer::LedState::Inactive,
                                            _ => crate::display_renderer::LedState::Active,
                                        };
                                        state.columns[slot_idx].button.exec = button_exec;
                                    }
                                }
                            }
                        }
                    }
                }
            }

            Some(updates) = led_rx.recv() => {
                let mut state = display_state.lock().unwrap();
                for (idx, is_active) in updates {
                    state.leds[idx] = if is_active {
                        crate::display_renderer::LedState::Active
                    } else {
                        crate::display_renderer::LedState::Inactive
                    };
                }
            }
        }
    }
}
