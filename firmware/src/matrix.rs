//! 11x11 Button Matrix Driver using 74HC595 and 74HC165 shift registers

use crate::protocol::BoardEvent;
use core::iter::Iterator;
use core::ops::FnMut;
use core::option::Option::{self, None, Some};
use stm32f4xx_hal::gpio::{Input, Output, PushPull, PA10, PB12, PB13, PB14, PB15};

pub const NUM_COLS: usize = 11;
pub const NUM_ROWS: usize = 11;
const TOTAL_SHIFT_BITS: usize = 16;
const DEBOUNCE_SAMPLES: u8 = 3;

fn bit_to_row(bit_idx: usize) -> Option<usize> {
    match bit_idx {
        7 => Some(0),   // U3 D0 -> R1 (Row 0)
        6 => Some(1),   // U3 D1 -> R2 (Row 1)
        5 => Some(2),   // U3 D2 -> R3 (Row 2)
        4 => Some(3),   // U3 D3 -> R4 (Row 3)
        3 => Some(4),   // U3 D4 -> R5 (Row 4)
        2 => Some(5),   // U3 D5 -> R6 (Row 5)
        1 => Some(6),   // U3 D6 -> R7 (Row 6)
        0 => Some(7),   // U3 D7 -> R8 (Row 7)
        15 => Some(8),  // U4 D0 -> R9 (Row 8)
        14 => Some(9),  // U4 D1 -> R10 (Row 9)
        13 => Some(10), // U4 D2 -> R11 (Row 10)
        _ => None,
    }
}

#[rustfmt::skip]
pub const RAW_TO_NOTE_MAP: [Option<u8>; 121] = [
    Some(62), Some(63), Some(64), Some(65), Some(66), Some(67), None,     Some(68), None,     None,     None,
    Some(53), Some(54), Some(55), Some(56), Some(57), Some(58), Some(59), Some(60), None,     None,     Some(61),
    Some(44), Some(45), Some(46), Some(47), Some(48), Some(49), Some(50), None,     None,     Some(52), Some(51),
    Some(36), Some(37), Some(38), Some(39), Some(40), Some(41), None,     None,     None,     Some(43), Some(42),
    Some(27), Some(28), Some(29), Some(30), Some(31), Some(32), Some(33), None,     None,     Some(35), Some(34),
    Some(18), Some(19), Some(20), Some(21), Some(22), Some(23), Some(24), None,     None,     Some(26), Some(25),
    Some(8),  Some(9),  Some(10), Some(11), Some(12), Some(13), Some(14), Some(15), None,     Some(17), Some(16),
    None,     None,     None,     Some(0),  Some(1),  Some(2),  Some(3),  Some(4),  Some(7),  Some(6),  Some(5),
    Some(92), Some(93), Some(87), Some(91), Some(90), Some(88), None,     None,     Some(89), None,     None,
    Some(79), Some(80), Some(81), Some(82), Some(83), Some(84), None,     Some(85), Some(86), None,     None,
    Some(69), Some(70), Some(71), Some(72), Some(73), Some(74), Some(75), Some(76), Some(78), None,     Some(77),
];

pub struct MatrixScanner {
    row_latch: PB12<Output<PushPull>>,
    sck: PB13<Output<PushPull>>,
    row_data: PB14<Input>,
    col_latch: PA10<Output<PushPull>>,
    col_data: PB15<Output<PushPull>>,
    stable_states: [[bool; NUM_COLS]; NUM_ROWS],
    debounce_counters: [[u8; NUM_COLS]; NUM_ROWS],
}

impl MatrixScanner {
    pub fn new(
        mut sck: PB13<Output<PushPull>>,
        mut row_latch: PB12<Output<PushPull>>,
        row_data: PB14<Input>,
        mut col_latch: PA10<Output<PushPull>>,
        mut col_data: PB15<Output<PushPull>>,
    ) -> Self {
        row_latch.set_high();
        sck.set_low();
        col_latch.set_low();
        col_data.set_low();

        Self {
            row_latch,
            sck,
            row_data,
            col_latch,
            col_data,
            stable_states: [[false; NUM_COLS]; NUM_ROWS],
            debounce_counters: [[0; NUM_COLS]; NUM_ROWS],
        }
    }

    /// Scan entire matrix once. Invokes callback `on_event` for any debounced state changes.
    pub fn scan<F>(&mut self, mut on_event: F)
    where
        F: FnMut(BoardEvent),
    {
        for col in 0..NUM_COLS {
            let col_mask = !(1u16 << col);

            // Shift 16 bits MSB-first into 74HC595
            for i in (0..TOTAL_SHIFT_BITS).rev() {
                let bit = (col_mask >> i) & 1;
                if bit == 1 {
                    self.col_data.set_high();
                } else {
                    self.col_data.set_low();
                }

                self.sck.set_high();
                cortex_m::asm::delay(4);
                self.sck.set_low();
                cortex_m::asm::delay(4);
            }

            // Latch 74HC595 output
            self.col_latch.set_high();
            cortex_m::asm::delay(4);
            self.col_latch.set_low();

            cortex_m::asm::delay(20);

            // Pulse ROWL (74HC165 PL# active low)
            self.row_latch.set_low();
            cortex_m::asm::delay(4);
            self.row_latch.set_high();
            cortex_m::asm::delay(4);

            // Read 16 bits from 74HC165 Q7
            for bit_idx in 0..TOTAL_SHIFT_BITS {
                let raw_pressed = self.row_data.is_low();

                if let Some(row) = bit_to_row(bit_idx) {
                    if raw_pressed != self.stable_states[row][col] {
                        self.debounce_counters[row][col] += 1;
                        if self.debounce_counters[row][col] >= DEBOUNCE_SAMPLES {
                            self.stable_states[row][col] = raw_pressed;
                            self.debounce_counters[row][col] = 0;

                            let raw_button_id = row * 11 + col;
                            if let Some(button_id) = RAW_TO_NOTE_MAP[raw_button_id] {
                                on_event(BoardEvent::MatrixButton { button_id, pressed: raw_pressed });
                            }
                        }
                    } else {
                        self.debounce_counters[row][col] = 0;
                    }
                }

                self.sck.set_high();
                cortex_m::asm::delay(4);
                self.sck.set_low();
                cortex_m::asm::delay(4);
            }
        }
    }
}
