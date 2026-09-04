//! 4 Quadrature Rotary Encoders & Push Buttons Module

use crate::protocol::BoardEvent;
use core::ops::FnMut;
use embedded_hal::Qei;
use stm32f4xx_hal::{
    gpio::{Input, PB8, PB9, PC13, PC14},
    pac::{TIM1, TIM2, TIM3, TIM4},
    qei::Qei as Stm32Qei,
};

pub struct EncoderSystem {
    qei0: Stm32Qei<TIM1>,
    qei1: Stm32Qei<TIM2>,
    qei2: Stm32Qei<TIM3>,
    qei3: Stm32Qei<TIM4>,

    btn0: PC13<Input>,
    btn1: PC14<Input>,
    btn2: PB9<Input>,
    btn3: PB8<Input>,

    last_counts: [u16; 4],
    last_btns: [bool; 4],
}

impl EncoderSystem {
    pub fn new(
        qei0: Stm32Qei<TIM1>,
        qei1: Stm32Qei<TIM2>,
        qei2: Stm32Qei<TIM3>,
        qei3: Stm32Qei<TIM4>,
        btn0: PC13<Input>,
        btn1: PC14<Input>,
        btn2: PB9<Input>,
        btn3: PB8<Input>,
    ) -> Self {
        let initial_counts = [qei0.count(), qei1.count() as u16, qei2.count(), qei3.count()];
        let initial_btns = [btn0.is_low(), btn1.is_low(), btn2.is_low(), btn3.is_low()];

        Self { qei0, qei1, qei2, qei3, btn0, btn1, btn2, btn3, last_counts: initial_counts, last_btns: initial_btns }
    }

    /// Read encoders and buttons, firing events for turns and button presses
    pub fn poll<F>(&mut self, mut on_event: F)
    where
        F: FnMut(BoardEvent),
    {
        let current_counts = [self.qei0.count(), self.qei1.count() as u16, self.qei2.count(), self.qei3.count()];

        let current_btns = [self.btn0.is_low(), self.btn1.is_low(), self.btn2.is_low(), self.btn3.is_low()];

        for i in 0..4 {
            // Check button state changes
            if current_btns[i] != self.last_btns[i] {
                self.last_btns[i] = current_btns[i];
                on_event(BoardEvent::EncoderButton { index: i as u8, pressed: current_btns[i] });
            }

            // Check encoder movement
            let diff = current_counts[i].wrapping_sub(self.last_counts[i]) as i16;
            // 4 pulses per detent typical on mechanical encoders
            if diff >= 4 || diff <= -4 {
                let detents = (diff / 4) as i8;
                if detents != 0 {
                    let cw = detents < 0; // Inverted direction
                    let count = detents.abs();
                    for _ in 0..count {
                        on_event(BoardEvent::EncoderTurn { index: i as u8, cw });
                    }
                    self.last_counts[i] = current_counts[i];
                }
            }
        }
    }
}
