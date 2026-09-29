# grandPA2

grandPA2 — the grandMA2 Physical Accessory — is a custom hardware controller inspired by the grandMA2 lighting console. It features a custom PCB, Rust-based STM32 firmware, and a companion Windows host application for grandMA2 WebRemote integration.

This is a private hobby project and is in no way affiliated with, endorsed by, or associated with MA Lighting Technology or the grandMA2 brand.

## Quick Start

To get the grandPA2 up and running, you need to build the firmware, configure your lighting software, and launch the host application:

1. **Flash Firmware:** Enter the firmware dev shell (`nix develop .#firmware`), build with `cargo build --release`, and flash to your Black Pill via `probe-rs` or `dfu-util`.
2. **Configure grandMA2 onPC:** Go to Global Settings and enable the **WebRemote**. Create a user account (default expected is username `grandpa2`, password `secret`).
3. **Launch Host App:** Enter the host dev shell (`nix develop .#host`), build with `cargo build --release --target x86_64-pc-windows-gnu`, and run the executable with your connection string: `grandpa2.exe grandpa2:secret@127.0.0.1:80`.

## Project Structure

- `firmware/` - Rust firmware for the STM32F4 (Black Pill) microcontroller.
- `host/` - Rust-based Windows companion application for screen scraping and two-way communication via grandMA2 WebRemote.
- `pcb/` - EasyEDA project and hardware design files.
- `keycaps/` - Python scripts to generate custom SVG legends for the keycaps.
- `casing/` - 3D models for the enclosure.

## Development Environment (Nix)

This project uses [Nix](https://nixos.org/) [Flakes](https://nixos.wiki/wiki/Flakes) to provide a reproducible development environment.

To enter the respective development environments with all necessary toolchains installed (Rust, cross-compilers, Python, Inkscape, etc.):

- **Firmware Dev Shell:** `nix develop .#firmware`
- **Windows Host App Dev Shell:** `nix develop .#host`
- **Keycap Generation Shell:** `nix develop .#keycaps`

---

## Hardware & PCB (`pcb/`)

The hardware is designed in EasyEDA. The `grandPA2.eprj` file can be imported into EasyEDA to view the schematics and PCB layout.

**Hardware Features:**
- STM32F411 (Black Pill) microcontroller
- 11x11 Switch Matrix (with 94 usable keys)
- 4x EC11 Rotary Encoders with push buttons
- 5x Analog Faders
- 4x OLED Displays (SH1106 via SPI)
- 94x WS2812 RGB LEDs

The button layout strictly follows the layout from a physical grandMA2 console, plus 16 extra buttons mapped as the X1 to X16 hardkeys. One fader acts as the Grand Master, while the other 4 faders are mapped as Executors 1-4. The buttons underneath the 4 faders are mapped as the main executor buttons for Executors 6-9.

The 4 rotary encoders are hardcoded to the following attributes:
- **Encoder 0:** Dimmer (Dim)
- **Encoder 1:** Position (Pan)
- **Encoder 2:** Position (Tilt)
- **Encoder 3:** Focus (Zoom)

The hardware is split into three separate boards for easier manufacturing and assembly:
1. **Main Board:** Hosts the Black Pill microcontroller and various IDC connectors to route signals.
2. **Matrix Board:** Contains the keyboard scanning matrix and the 94 RGB LEDs. It also contains a smaller sub-PCB for the executor buttons.
3. **Peripheral Boards:** Smaller boards, each used for connecting one display, one encoder, and one fader.

### MCU Pin Assignments
The Black Pill (STM32F411) uses the following pins for the various peripherals:

| Peripheral | Sub-component / Signal | Pin(s) |
| :--- | :--- | :--- |
| **Encoders** | Encoder 0 (A, B, Push) | `PA8`, `PA9`, `PC13` |
| | Encoder 1 (A, B, Push) | `PA15`, `PB3`, `PC14` |
| | Encoder 2 (A, B, Push) | `PB4`, `PB5`, `PB9` |
| | Encoder 3 (A, B, Push) | `PB6`, `PB7`, `PB8` |
| **Faders** | Faders 0 - 4 | `PA0`, `PA1`, `PA2`, `PA3`, `PA4` |
| **Matrix Scanner** | SCK, Row Latch, Row Data In | `PB13`, `PB12`, `PB14` |
| | Col Latch, Col Data Out | `PA10`, `PB15` |
| **Displays (SPI)** | SCK, MOSI, DC | `PA5`, `PA7`, `PC15` |
| | CS0, CS1, CS2, CS3 | `PB0`, `PB1`, `PB2`, `PB10` |
| **WS2812 LEDs** | Data Line | `PA6` |

### IDC Connector Pinouts (Main Board)
The main board routes signals to the peripherals via IDC connectors. The pinouts are as follows:

**Matrix (10-pin)**
| Odd Pin | Signal | Even Pin | Signal |
| :--- | :--- | :--- | :--- |
| **1** | `3V3` | **2** | `ROW_LATCH` |
| **3** | `SCK` | **4** | `ROW_DATA` |
| **5** | `COL_LATCH` | **6** | `COL_DATA` |
| **7** | `GND` | **8** | `GND` |
| **9** | `GND` | **10** | `GND` |

**Displays (10-pin)**
| Odd Pin | Signal | Even Pin | Signal |
| :--- | :--- | :--- | :--- |
| **1** | `CS` | **2** | `MOSI` |
| **3** | `3V3` | **4** | `DC` |
| **5** | `GND` | **6** | `RST` |
| **7** | `GND` | **8** | `SCK` |
| **9** | `GND` | **10** | `GND` |

**Encoders (6-pin)**
| Odd Pin | Signal | Even Pin | Signal |
| :--- | :--- | :--- | :--- |
| **1** | `ENC_A` | **2** | `ENC_BTN` |
| **3** | `ENC_B` | **4** | `GND` |
| **5** | `GND` | **6** | `GND` |

**Faders (6-pin)**
| Odd Pin | Signal | Even Pin | Signal |
| :--- | :--- | :--- | :--- |
| **1** | `3V3` | **2** | `FADER` |
| **3** | `GND` | **4** | `GND` |
| **5** | `GND` | **6** | `GND` |

**LEDs (6-pin)**
| Odd Pin | Signal | Even Pin | Signal |
| :--- | :--- | :--- | :--- |
| **1** | `5V` | **2** | `5V` |
| **3** | `LED_DATA` | **4** | `GND` |
| **5** | `GND` | **6** | `GND` |

### Bill of Materials (BOM)
In addition to the components listed in the PCB design, the following components are required:
- 1x STM32F411 Black Pill board
- 94x MX button switches
- 90x 1U keycaps
- 4x 2U keycaps
- 4x EC11 rotary encoders
- 5x linear faders (128mm travel, e.g., 10k linear)
- Various 6P, 10P, and 16P IDC sockets, connectors, and ribbon cables for inter-board connections.
- M3 8mm screws (for the casing).
- Self-cutting screws (for mounting the PCB).

### Keycaps & Legends (`keycaps/`)
Custom keycaps are required to mimic the lighting console layout. The `generate.py` script generates perfectly aligned, spaced SVGs for standard 1U and 2U keys, complete with a custom font and logos.
- To generate: `nix develop .#keycaps`, then `python generate.py`.
- The script uses Inkscape via the CLI to automatically convert the text elements into SVG paths, ensuring they render correctly for printing or laser engraving.

*Tip: For the best results, use transparent keycaps sprayed with matte black paint, and then laser-ablate the labels.*

### Casing (`casing/`)
The 3D printed casing for grandPA2 is designed in Onshape. You can view, copy, and export the CAD models from the following link:
[grandPA2 Casing on Onshape](https://cad.onshape.com/documents/a69522914b9fd6c3a378e748/w/ff3ce6cc18ab1158cfabc037/e/642326b3695c8e1d442461fe)
Exported STEP files are also in the repository. There is a "split" version that fits into the 256mm³ print volume of a Bambu Lab P1S printer.

---

## Firmware (`firmware/`)

The firmware is written in Rust for the STM32F411 (Black Pill) microcontroller. It acts as a USB MIDI Audio Class device.

### Protocol Details
The firmware and the host software communicate via standard USB MIDI, mixed with custom SysEx commands:

- **Hardware to Host (Standard MIDI):**
  - **Matrix Buttons:** Sent as MIDI Note On/Off messages (Notes 0–120) on Channel 1.
  - **Encoders:** The rotation is sent as Control Change (CC) messages (CC 0–3, relative 2's complement) on Channel 2. The encoder push buttons send Note On/Off messages (Notes 0–3) on Channel 2.
  - **Faders:** Sent as absolute Control Change (CC) messages (CC 0–4) on Channel 3.
  
- **Host to Hardware (Custom SysEx - ID 0x7D):**
  - **LED Updates (Command ID `0x10`):** Host sends 24-bit RGB colors for all 94 LEDs. The payload is 282 bytes (94 LEDs * 3 bytes for R, G, B). Each byte is 7-bit masked.
  - **OLED Displays (Command ID `0x20` to `0x2F`):** Host sends 1BPP pixel chunks. DisplayID is `(Cmd - 0x20) / 4` and ChunkID is `(Cmd - 0x20) % 4`. The payload is 7-bit packed and unpacked by the firmware into 256 bytes per chunk.

SysEx messages are wrapped in standard MIDI `0xF0` and `0xF7` bytes. The payload begins with the Custom Manufacturer ID `0x7D`, followed by the Command ID, and then the data payload bytes (which are strictly 7-bit compliant).

### Building and Flashing
1. Enter the firmware dev shell: `nix develop .#firmware`
2. Build the project: `cargo build --release`
3. Flash the firmware onto the Black Pill using `probe-rs` or `dfu-util`.

---

## Host Software (`host/`)

The host software is a Windows application written in Rust. It acts as the bridge between the grandPA2 hardware and the grandMA2 onPC software by combining screen scraping with WebRemote integration.


### grandMA2 WebRemote Integration

The host uses websockets to communicate with the grandMA2 WebRemote API. This handles bidirectional data transfer:
- **Fetching State:** Continuously requests `PresetTypesUpdate` and `PlaybacksUpdate` updates to render accurate encoder values and fader / button states onto the OLED displays.
- **Sending Commands:** Translates physical encoder turns and hardkey presses into WebRemote JSON commands (e.g., `PresetType`, `HardKey`, `Encoder`).

### Screen Scraping for LED Feedback

Because grandMA2 onPC doesn't expose a native API to read the LED state of the on-screen buttons, the host software falls back to screen scraping. 

It uses Windows Graphics Capture (DXGI) and Direct3D11 to capture frames from the grandMA2 onPC window in real-time. It checks the colors of specific pixels at normalized coordinates mapped to the grandMA2 UI to determine if a button is active/lit. These active states are translated into the custom SysEx protocol and sent to the microcontroller, lighting up the physical WS2812 RGB LEDs.

### Building
The Nix environment provides a MinGW-w64 cross-compiler if you are building from Linux.
1. Enter the host dev shell: `nix develop .#host`
2. Build the project: `cargo build --release --target x86_64-pc-windows-gnu`

If you are on Windows natively, you can build using the MSVC toolchain with standard `cargo build --release`.