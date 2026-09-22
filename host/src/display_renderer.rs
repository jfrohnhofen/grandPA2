use embedded_graphics::{
    mono_font::{ascii::FONT_8X13, MonoTextStyle},
    pixelcolor::BinaryColor,
    prelude::*,
    text::Text,
};
use midir::MidiOutputConnection;
use std::sync::{Arc, Mutex};

pub const SYSEX_MANUFACTURER_ID: u8 = 0x7D;
use crate::keymap::NUM_LEDS;

use crate::grandma2_client::{ButtonExec, ButtonFunction, FaderExec, FaderFunction};

#[derive(Clone, Copy, PartialEq)]
pub enum LedState {
    Off,
    Inactive,
    Active,
}

impl LedState {
    pub fn color(&self) -> (u8, u8, u8) {
        match self {
            LedState::Off => (0, 0, 0),
            LedState::Inactive => (40, 14, 0), // Dim Orange idle
            LedState::Active => (255, 80, 0),  // Bright Orange active
        }
    }
}

#[derive(Clone, PartialEq, Default)]
pub struct EncoderState {
    pub value: String,
    pub active: bool,
    pub programmer: bool,
    pub fine_mode: bool,
    pub group_active: bool,
}

#[derive(Clone, PartialEq, Default)]
pub struct FaderState {
    pub value: u8,
    pub exec: FaderExec,
}

#[derive(Clone, PartialEq, Default)]
pub struct ButtonState {
    pub is_pressed: bool,
    pub flash_time: Option<std::time::Instant>,
    pub exec: ButtonExec,
}

#[derive(Clone, PartialEq, Default)]
pub struct Column {
    pub encoder: EncoderState,
    pub fader: FaderState,
    pub button: ButtonState,
}

#[derive(Clone, PartialEq)]
pub struct DisplayState {
    pub leds: [LedState; NUM_LEDS],
    pub columns: [Column; 4],
    pub force_redraw: bool,
}

impl DisplayState {
    pub fn new() -> Self {
        Self { leds: [LedState::Inactive; NUM_LEDS], columns: Default::default(), force_redraw: true }
    }
}

pub fn start(display_state: Arc<Mutex<DisplayState>>, midi_handle: Arc<Mutex<Option<MidiOutputConnection>>>) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(50));
        let mut prev_state = DisplayState::new();

        loop {
            interval.tick().await;

            let (current_state, force) = {
                let mut state = display_state.lock().unwrap();
                let force = state.force_redraw;
                state.force_redraw = false;

                // Clear flash_time after 250ms to automatically trigger a redraw
                for disp_idx in 0..4 {
                    if let Some(t) = state.columns[disp_idx].button.flash_time {
                        if t.elapsed().as_millis() > 250 {
                            state.columns[disp_idx].button.flash_time = None;
                        }
                    }
                }

                (state.clone(), force)
            };

            let mut chunks_to_send = Vec::new();

            if force || current_state.leds != prev_state.leds {
                chunks_to_send.push(create_led_sysex(&current_state.leds));
            }

            // Draw displays 0-3 (Encoders)
            for disp_idx in 0..4 {
                if force || current_state.columns[disp_idx] != prev_state.columns[disp_idx] {
                    let chunks = render_display_chunks(disp_idx as u8, &current_state);
                    chunks_to_send.extend(chunks);
                }
            }

            if !chunks_to_send.is_empty() {
                let mut handle = midi_handle.lock().unwrap();
                if let Some(conn_out) = handle.as_mut() {
                    for chunk in chunks_to_send {
                        let _ = conn_out.send(&chunk);
                        // Brief pause to prevent USB overwhelming
                        std::thread::sleep(std::time::Duration::from_millis(1));
                    }
                }
            }

            prev_state = current_state;
        }
    });
}

pub fn create_led_sysex(states: &[LedState; NUM_LEDS]) -> Vec<u8> {
    let mut sysex = Vec::with_capacity(3 + NUM_LEDS * 3 + 1);
    sysex.push(0xF0);
    sysex.push(SYSEX_MANUFACTURER_ID);
    sysex.push(0x10);

    for state in states.iter() {
        let (r, g, b) = state.color();
        sysex.push(r >> 1);
        sysex.push(g >> 1);
        sysex.push(b >> 1);
    }

    sysex.push(0xF7);
    sysex
}

fn create_oled_sysex_chunks(disp_idx: u8, fb: &[u8]) -> Vec<Vec<u8>> {
    let mut chunks = Vec::with_capacity(4);
    for chunk_id in 0..4 {
        let cmd_id = 0x20 + (disp_idx * 4) + chunk_id;
        let start = (chunk_id as usize) * 256;
        let chunk_bytes = &fb[start..start + 256];

        let packed_7bit = pack_8to7(chunk_bytes);

        let mut sysex = Vec::with_capacity(3 + packed_7bit.len() + 1);
        sysex.push(0xF0);
        sysex.push(SYSEX_MANUFACTURER_ID);
        sysex.push(cmd_id);
        sysex.extend_from_slice(&packed_7bit);
        sysex.push(0xF7);

        chunks.push(sysex);
    }

    chunks
}

pub fn render_display_chunks(disp_idx: u8, state: &DisplayState) -> Vec<Vec<u8>> {
    let disp = (disp_idx & 0x03) as usize;
    let enc_names = ["DIM", "PAN", "TILT", "ZOOM"];

    let mut canvas = [0u8; 1024];

    let mut draw_pixel = |x: i32, y: i32, on: bool| {
        if x >= 0 && x < 128 && y >= 0 && y < 64 {
            let page = (y as usize) / 8;
            let bit_pos = (y as usize) % 8;
            let byte_idx = (page * 128) + (x as usize);
            let bit_mask = 1 << bit_pos;
            if on {
                canvas[byte_idx] |= bit_mask;
            } else {
                canvas[byte_idx] &= !bit_mask;
            }
        }
    };

    // Draw encoder zone (rows 1-20)

    // Background fill for programmer active (invert rows 2-19, cols 4-123)
    let is_prog = state.columns[disp].encoder.programmer;
    if is_prog {
        for y in 2..20 {
            for x in 4..124 {
                draw_pixel(x, y, true);
            }
        }
    }

    let mut draw_top = |x: i32, y: i32, mut on: bool| {
        if is_prog && x >= 4 && x < 124 && y >= 2 && y < 20 {
            on = !on;
        }
        draw_pixel(x, y, on);
    };

    // Render on a single line, vertically centered in the 1-20 area (baseline ~14)
    let disp_name = enc_names[disp];
    let left_text = format!("{} {}", disp_name, state.columns[disp].encoder.value);
    render_text(&left_text, 4, 14, &mut draw_top, FONT_8X13);

    let fine_char = if state.columns[disp].encoder.fine_mode { "F" } else { "C" };
    let right_text = format!("[{}]", fine_char);
    render_text(&right_text, 100, 14, &mut draw_top, FONT_8X13);

    // Separator line at Y=21
    for x in 4..124 {
        draw_pixel(x, 21, true);
    }

    // Draw fader zone (rows 22-41)
    let fader_exec = &state.columns[disp].fader.exec;
    let fader_val = state.columns[disp].fader.value;
    let fader_pct = (fader_val as f32 / 127.0 * 100.0) as u32;

    if !fader_exec.name.is_empty() {
        let is_fader_active = fader_exec.active || fader_val > 0;
        let fill_width = match (is_fader_active, &fader_exec.function, fader_exec.fade) {
            (false, _, _) => 0,
            (true, FaderFunction::Temp, _) => 120,
            (true, _, Some(fade)) if fade > 0.0 && fade < 1.0 => (fade * 120.0) as i32,
            _ => 120,
        };

        if is_fader_active {
            for y in 23..41 {
                for x in 4..(4 + fill_width) {
                    draw_pixel(x, y, true);
                }
            }
        }

        let mut draw_fader = |x: i32, y: i32, mut on: bool| {
            if is_fader_active && x >= 4 && x < (4 + fill_width) && y >= 23 && y < 41 {
                on = !on;
            }
            draw_pixel(x, y, on);
        };

        let fader_title = fader_exec.name.clone();
        let fader_title_short = fader_title.chars().take(6).collect::<String>();
        render_text(&fader_title_short, 4, 35, &mut draw_fader, FONT_8X13);

        if !fader_exec.cue.is_empty() && fader_exec.function != FaderFunction::Temp {
            let cue_str = fader_exec.cue.trim();
            render_text(
                &format!("{:>8}", cue_str.chars().take(8).collect::<String>()),
                60,
                35,
                &mut draw_fader,
                FONT_8X13,
            );
        } else {
            render_text(&format!("{:>8}", format!("{:3}%", fader_pct)), 60, 35, &mut draw_fader, FONT_8X13);
        }

        // Draw horizontal bar graph for fader level
        let bar_pct = (fader_val as f32 / 127.0).clamp(0.0, 1.0);
        let bar_width = (bar_pct * 120.0) as i32;
        for y in 37..41 {
            for x in 4..(4 + bar_width) {
                draw_fader(x, y, true);
            }
        }
    }

    // Separator line at Y=42
    for x in 4..124 {
        draw_pixel(x, 42, true);
    }

    // Draw button zone (rows 43-62)
    let button_state = &state.columns[disp].button;
    let button_exec = &button_state.exec;

    if !button_exec.name.is_empty() && button_exec.function != ButtonFunction::Empty {
        let mut is_button_active = button_exec.active;
        let is_toggle = button_exec.function == ButtonFunction::Toggle;

        if !is_toggle {
            let is_physically_pressed = button_state.is_pressed;
            let recently_pressed =
                if let Some(t) = button_state.flash_time { t.elapsed().as_millis() < 250 } else { false };

            if is_physically_pressed || recently_pressed {
                is_button_active = true;
            }
        }
        if is_button_active {
            for y in 44..62 {
                for x in 4..124 {
                    draw_pixel(x, y, true);
                }
            }
        }

        let mut draw_button = |x: i32, y: i32, mut on: bool| {
            if is_button_active && x >= 4 && x < 124 && y >= 44 && y < 62 {
                on = !on;
            }
            draw_pixel(x, y, on);
        };

        let button_title = button_exec.name.clone();
        let button_title_short = button_title.chars().take(6).collect::<String>();
        render_text(&button_title_short, 4, 56, &mut draw_button, FONT_8X13);

        let label = match &button_exec.function {
            ButtonFunction::Toggle => {
                if button_exec.active {
                    "On".to_string()
                } else {
                    "Off".to_string()
                }
            }
            ButtonFunction::Flash | ButtonFunction::Temp => "Flash".to_string(),
            ButtonFunction::Learn => button_exec.cue_name.clone().unwrap_or_default(),
            ButtonFunction::Other(s) => s.clone(),
            _ => "".to_string(),
        };

        if !label.is_empty() {
            render_text(
                &format!("{:>8}", label.chars().take(8).collect::<String>()),
                60,
                56,
                &mut draw_button,
                FONT_8X13,
            );
        }
    }

    create_oled_sysex_chunks(disp_idx, &canvas)
}

fn render_text<F: FnMut(i32, i32, bool)>(
    text: &str,
    start_x: i32,
    start_y: i32,
    draw_pixel: &mut F,
    font: embedded_graphics::mono_font::MonoFont,
) {
    let style = MonoTextStyle::new(&font, BinaryColor::On);

    struct CanvasTarget<'a, F: FnMut(i32, i32, bool)> {
        draw_pixel: &'a mut F,
    }

    impl<'a, F: FnMut(i32, i32, bool)> DrawTarget for CanvasTarget<'a, F> {
        type Color = BinaryColor;
        type Error = std::convert::Infallible;

        fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
        where
            I: IntoIterator<Item = Pixel<Self::Color>>,
        {
            for Pixel(point, color) in pixels {
                (self.draw_pixel)(point.x, point.y, color.is_on());
            }
            Ok(())
        }
    }

    impl<'a, F: FnMut(i32, i32, bool)> OriginDimensions for CanvasTarget<'a, F> {
        fn size(&self) -> Size {
            Size::new(128, 64)
        }
    }

    let mut target = CanvasTarget { draw_pixel };
    let _ = Text::new(text, Point::new(start_x, start_y), style).draw(&mut target);
}

fn pack_8to7(input: &[u8]) -> Vec<u8> {
    let mut output = Vec::with_capacity((input.len() * 8 + 6) / 7);
    let mut bit_buf: u32 = 0;
    let mut bits_in_buf: u8 = 0;

    for &byte in input {
        bit_buf = (bit_buf << 8) | (byte as u32);
        bits_in_buf += 8;

        while bits_in_buf >= 7 {
            bits_in_buf -= 7;
            let val_7bit = ((bit_buf >> bits_in_buf) & 0x7F) as u8;
            output.push(val_7bit);
            if bits_in_buf > 0 {
                bit_buf &= (1 << bits_in_buf) - 1;
            } else {
                bit_buf = 0;
            }
        }
    }

    if bits_in_buf > 0 {
        let val_7bit = ((bit_buf << (7 - bits_in_buf)) & 0x7F) as u8;
        output.push(val_7bit);
    }

    output
}
