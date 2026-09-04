#![no_std]
#![no_main]

use cortex_m_rt::entry;
use panic_halt as _;

use stm32f4xx_hal::{
    adc::{config::AdcConfig, Adc},
    gpio::{
        alt::otg_fs::{Dm, Dp},
        Output, PushPull, PA6,
    },
    otg_fs::{UsbBus, USB},
    pac,
    prelude::*,
    qei::Qei,
};

use usb_device::prelude::*;
use usbd_midi::UsbMidiClass;

use grandpa2_firmware::{
    displays::DisplaySystem, encoders::EncoderSystem, faders::FaderSystem, leds::LedSystem, matrix::MatrixScanner,
    protocol::HostCommand, usb_midi::UsbMidiManager,
};

#[entry]
fn main() -> ! {
    let dp = pac::Peripherals::take().unwrap();

    let rcc = dp.RCC.constrain();
    let clocks = rcc.cfgr.sysclk(48.MHz()).require_pll48clk().freeze();

    // Give external components (OLEDs, WS2812s) time to power up before sending init commands.
    // At 48MHz, 12,000,000 cycles is roughly 250ms.
    cortex_m::asm::delay(12_000_000);

    let gpioa = dp.GPIOA.split();
    let gpiob = dp.GPIOB.split();
    let gpioc = dp.GPIOC.split();

    // 1. USB MIDI Setup
    let usb_dm = gpioa.pa11.into_alternate();
    let usb_dp = gpioa.pa12.into_alternate();

    let usb = USB {
        usb_global: dp.OTG_FS_GLOBAL,
        usb_device: dp.OTG_FS_DEVICE,
        usb_pwrclk: dp.OTG_FS_PWRCLK,
        pin_dm: Dm::PA11(usb_dm),
        pin_dp: Dp::PA12(usb_dp),
        hclk: clocks.hclk(),
    };

    static mut EP_MEMORY: [u32; 1024] = [0; 1024];
    #[allow(static_mut_refs)]
    let usb_bus = UsbBus::new(usb, unsafe { &mut EP_MEMORY });

    let midi_class = UsbMidiClass::new(&usb_bus, 1, 1).unwrap();
    let usb_dev = UsbDeviceBuilder::new(&usb_bus, UsbVidPid(0x1209, 0x0001))
        .strings(&[StringDescriptors::default().manufacturer("grandPA2").product("grandPA2").serial_number("000001")])
        .unwrap()
        .device_class(0x01) // USB_CLASS_AUDIO
        .build();

    let mut usb_midi = UsbMidiManager::new(usb_dev, midi_class);

    // 2. Rotary Encoders Setup
    let enc0_a = gpioa.pa8.into_alternate();
    let enc0_b = gpioa.pa9.into_alternate();
    let qei0 = Qei::new(dp.TIM1, (enc0_a, enc0_b));

    let enc1_a = gpioa.pa15.into_alternate();
    let enc1_b = gpiob.pb3.into_alternate();
    let qei1 = Qei::new(dp.TIM2, (enc1_a, enc1_b));

    let enc2_a = gpiob.pb4.into_alternate();
    let enc2_b = gpiob.pb5.into_alternate();
    let qei2 = Qei::new(dp.TIM3, (enc2_a, enc2_b));

    let enc3_a = gpiob.pb6.into_alternate();
    let enc3_b = gpiob.pb7.into_alternate();
    let qei3 = Qei::new(dp.TIM4, (enc3_a, enc3_b));

    let enc_btn0 = gpioc.pc13.into_floating_input();
    let enc_btn1 = gpioc.pc14.into_floating_input();
    let enc_btn2 = gpiob.pb9.into_floating_input();
    let enc_btn3 = gpiob.pb8.into_floating_input();

    let mut encoders = EncoderSystem::new(qei0, qei1, qei2, qei3, enc_btn0, enc_btn1, enc_btn2, enc_btn3);

    // 3. Faders ADC Setup
    let fader0 = gpioa.pa0.into_analog();
    let fader1 = gpioa.pa1.into_analog();
    let fader2 = gpioa.pa2.into_analog();
    let fader3 = gpioa.pa3.into_analog();
    let fader4 = gpioa.pa4.into_analog();
    let adc = Adc::adc1(dp.ADC1, true, AdcConfig::default());

    let mut faders = FaderSystem::new(adc, fader0, fader1, fader2, fader3, fader4);

    // 4. Matrix Scanner Setup
    let sck = gpiob.pb13.into_push_pull_output();
    let row_latch = gpiob.pb12.into_push_pull_output();
    let row_data = gpiob.pb14.into_floating_input();
    let col_latch = gpioa.pa10.into_push_pull_output();
    let col_data = gpiob.pb15.into_push_pull_output();

    let mut matrix = MatrixScanner::new(sck, row_latch, row_data, col_latch, col_data);

    // 5. Displays Setup
    let dc = gpioc.pc15.into_push_pull_output();
    let cs0 = gpiob.pb0.into_push_pull_output();
    let cs1 = gpiob.pb1.into_push_pull_output();
    let cs2 = gpiob.pb2.into_push_pull_output();
    let cs3 = gpiob.pb10.into_push_pull_output();

    let mut displays = DisplaySystem::new(dp.SPI1, gpioa.pa5, gpioa.pa7, dc, cs0, cs1, cs2, cs3, &clocks);

    // 6. WS2812 RGB LEDs Setup
    let led_pin: PA6<Output<PushPull>> = gpioa.pa6.into_push_pull_output();
    let mut leds = LedSystem::new(led_pin);

    loop {
        // A. Poll USB MIDI for incoming commands from grandMA2
        usb_midi.poll(|cmd| match cmd {
            HostCommand::SetAllRgbLeds { colors } => {
                leds.set_all_rgb_7bit(&colors);
            }
            HostCommand::SetDisplayPixelChunk { display_id, chunk_id, pixels } => {
                displays.set_pixel_chunk(display_id, chunk_id, &pixels);
            }
        });

        // B. Scan 11x11 Matrix -> Send MIDI Note events to PC
        matrix.scan(|event| {
            usb_midi.send_event(event);
        });

        // C. Poll Encoders & Buttons -> Send MIDI CC / Note events to PC
        encoders.poll(|event| {
            usb_midi.send_event(event);
        });

        // D. Poll Analog Faders -> Send MIDI CC events to PC
        faders.poll(|event| {
            usb_midi.send_event(event);
        });

        // E. Flush Display & LED updates
        displays.update();
        leds.update();
    }
}
