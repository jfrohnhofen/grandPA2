#![no_std]
#![no_main]

use panic_halt as _;
use cortex_m_rt::entry;
use stm32f4xx_hal::{pac, prelude::*};

#[entry]
fn main() -> ! {
    // Take ownership of the device peripherals
    let dp = pac::Peripherals::take().unwrap();
    // Take ownership of the core peripherals
    let cp = cortex_m::Peripherals::take().unwrap();

    // Constrain clock registers
    let rcc = dp.RCC.constrain();
    // Configure the clocks to run at 48 MHz
    let clocks = rcc.cfgr.sysclk(48.MHz()).freeze();

    // Acquire the GPIOC peripheral
    let gpioc = dp.GPIOC.split();
    
    // Configure the PC13 pin as a push-pull output.
    // The onboard LED on the Black Pill is connected to PC13 and is active low.
    let mut led = gpioc.pc13.into_push_pull_output();

    // Get a delay provider using the SysTick timer
    let mut delay = cp.SYST.delay(&clocks);

    loop {
        led.set_low(); // LED on
        delay.delay_ms(500_u32);
        
        led.set_high(); // LED off
        delay.delay_ms(500_u32);
    }
}
