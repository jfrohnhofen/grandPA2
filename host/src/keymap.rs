//! grandPA2 Master Key Mapping Table
//! Indexed directly by idx (0..93) matching WS2812 LED positions and MIDI notes.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyFunction {
    /// Standard grandMA2 CANBUS hardkey (code 0..255)
    HardKey(u8),

    /// Executor button (e.g. Exec 1..4)
    ExecButton(u8),
}

#[derive(Debug, Clone, Copy)]
pub struct KeyConfig {
    pub function: KeyFunction,
    pub norm_coords: Option<(f64, f64)>,
}

pub const NUM_LEDS: usize = MASTER_KEY_MAP.len();

pub const MASTER_KEY_MAP: [KeyConfig; 94] = [
    // Row 1
    KeyConfig { function: KeyFunction::HardKey(119), norm_coords: Some((0.77790, 0.02857)) }, // TOOLS
    KeyConfig { function: KeyFunction::HardKey(118), norm_coords: Some((0.81849, 0.02857)) }, // SETUP
    KeyConfig { function: KeyFunction::HardKey(117), norm_coords: Some((0.85795, 0.02857)) }, // BACKUP
    KeyConfig { function: KeyFunction::HardKey(12), norm_coords: None },                      // X1
    KeyConfig { function: KeyFunction::HardKey(13), norm_coords: None },                      // X2
    KeyConfig { function: KeyFunction::HardKey(14), norm_coords: None },                      // X3
    KeyConfig { function: KeyFunction::HardKey(15), norm_coords: None },                      // X4
    KeyConfig { function: KeyFunction::HardKey(65), norm_coords: Some((0.92221, 0.44381)) },  // BLACKOUT
    // Row 2
    KeyConfig { function: KeyFunction::HardKey(60), norm_coords: Some((0.61838, 0.08762)) }, // BLIND
    KeyConfig { function: KeyFunction::HardKey(61), norm_coords: Some((0.65896, 0.08762)) }, // FREEZE
    KeyConfig { function: KeyFunction::HardKey(62), norm_coords: Some((0.69899, 0.08762)) }, // PRVW
    KeyConfig { function: KeyFunction::HardKey(63), norm_coords: Some((0.77790, 0.08762)) }, // ASSIGN
    KeyConfig { function: KeyFunction::HardKey(64), norm_coords: Some((0.81849, 0.08762)) }, // ALIGN
    KeyConfig { function: KeyFunction::HardKey(116), norm_coords: Some((0.85795, 0.08762)) }, // HELP
    KeyConfig { function: KeyFunction::HardKey(16), norm_coords: None },                     // X5
    KeyConfig { function: KeyFunction::HardKey(17), norm_coords: None },                     // X6
    KeyConfig { function: KeyFunction::HardKey(18), norm_coords: None },                     // X7
    KeyConfig { function: KeyFunction::HardKey(19), norm_coords: None },                     // X8
    // Row 3
    KeyConfig { function: KeyFunction::HardKey(41), norm_coords: Some((0.61838, 0.16286)) }, // FIX
    KeyConfig { function: KeyFunction::HardKey(42), norm_coords: Some((0.65896, 0.16286)) }, // SELECT
    KeyConfig { function: KeyFunction::HardKey(43), norm_coords: Some((0.69899, 0.16286)) }, // OFF
    KeyConfig { function: KeyFunction::HardKey(66), norm_coords: Some((0.77790, 0.16286)) }, // VIEW
    KeyConfig { function: KeyFunction::HardKey(67), norm_coords: Some((0.81849, 0.16286)) }, // EFFECT
    KeyConfig { function: KeyFunction::HardKey(56), norm_coords: Some((0.85795, 0.16286)) }, // GOTO
    KeyConfig { function: KeyFunction::HardKey(69), norm_coords: Some((0.92221, 0.16286)) }, // DELETE
    KeyConfig { function: KeyFunction::HardKey(20), norm_coords: None },                     // X9
    KeyConfig { function: KeyFunction::HardKey(21), norm_coords: None },                     // X10
    // Row 4
    KeyConfig { function: KeyFunction::HardKey(44), norm_coords: Some((0.61838, 0.23048)) }, // TEMP
    KeyConfig { function: KeyFunction::HardKey(45), norm_coords: Some((0.65896, 0.23048)) }, // TOP
    KeyConfig { function: KeyFunction::HardKey(46), norm_coords: Some((0.69899, 0.23048)) }, // ON
    KeyConfig { function: KeyFunction::HardKey(70), norm_coords: Some((0.77790, 0.23048)) }, // PAGE
    KeyConfig { function: KeyFunction::HardKey(71), norm_coords: Some((0.81849, 0.23048)) }, // MACRO
    KeyConfig { function: KeyFunction::HardKey(72), norm_coords: Some((0.85795, 0.23048)) }, // PRESET
    KeyConfig { function: KeyFunction::HardKey(73), norm_coords: Some((0.92221, 0.26429)) }, // COPY
    KeyConfig { function: KeyFunction::HardKey(22), norm_coords: None },                     // X11
    KeyConfig { function: KeyFunction::HardKey(23), norm_coords: None },                     // X12
    // Row 5
    KeyConfig { function: KeyFunction::HardKey(47), norm_coords: Some((0.61838, 0.29810)) }, // <<<
    KeyConfig { function: KeyFunction::HardKey(48), norm_coords: Some((0.65896, 0.29810)) }, // LEARN
    KeyConfig { function: KeyFunction::HardKey(49), norm_coords: Some((0.69899, 0.29810)) }, // >>>
    KeyConfig { function: KeyFunction::HardKey(74), norm_coords: Some((0.77790, 0.29810)) }, // SEQU
    KeyConfig { function: KeyFunction::HardKey(75), norm_coords: Some((0.81849, 0.29810)) }, // CUE
    KeyConfig { function: KeyFunction::HardKey(76), norm_coords: Some((0.85795, 0.29810)) }, // EXEC
    KeyConfig { function: KeyFunction::HardKey(24), norm_coords: None },                     // X13
    KeyConfig { function: KeyFunction::HardKey(25), norm_coords: None },                     // X14
    // Row 6
    KeyConfig { function: KeyFunction::HardKey(50), norm_coords: Some((0.61838, 0.36571)) }, // GO-
    KeyConfig { function: KeyFunction::HardKey(51), norm_coords: Some((0.65896, 0.36571)) }, // PAUSE
    KeyConfig { function: KeyFunction::HardKey(52), norm_coords: Some((0.69899, 0.36571)) }, // GO+
    KeyConfig { function: KeyFunction::HardKey(82), norm_coords: Some((0.77790, 0.36571)) }, // CHANNEL
    KeyConfig { function: KeyFunction::HardKey(83), norm_coords: Some((0.81849, 0.36571)) }, // FIXTURE
    KeyConfig { function: KeyFunction::HardKey(84), norm_coords: Some((0.85795, 0.36571)) }, // GROUP
    KeyConfig { function: KeyFunction::HardKey(85), norm_coords: Some((0.92221, 0.36571)) }, // MOVE
    KeyConfig { function: KeyFunction::HardKey(26), norm_coords: None },                     // X15
    KeyConfig { function: KeyFunction::HardKey(27), norm_coords: None },                     // X16
    // Row 7
    KeyConfig { function: KeyFunction::HardKey(58), norm_coords: Some((0.61838, 0.44381)) }, // TIME
    KeyConfig { function: KeyFunction::HardKey(54), norm_coords: Some((0.65896, 0.44381)) }, // ESC
    KeyConfig { function: KeyFunction::HardKey(93), norm_coords: None },                     // 7
    KeyConfig { function: KeyFunction::HardKey(94), norm_coords: None },                     // 8
    KeyConfig { function: KeyFunction::HardKey(95), norm_coords: None },                     // 9
    KeyConfig { function: KeyFunction::HardKey(96), norm_coords: None },                     // +
    KeyConfig { function: KeyFunction::HardKey(99), norm_coords: Some((0.77790, 0.78571)) }, // FULL
    KeyConfig { function: KeyFunction::HardKey(100), norm_coords: Some((0.81849, 0.78571)) }, // HIGH
    KeyConfig { function: KeyFunction::HardKey(101), norm_coords: Some((0.85795, 0.78571)) }, // SOLO
    // Row 8
    KeyConfig { function: KeyFunction::HardKey(55), norm_coords: Some((0.61838, 0.52000)) }, // EDIT
    KeyConfig { function: KeyFunction::HardKey(53), norm_coords: None },                     // OOPS
    KeyConfig { function: KeyFunction::HardKey(90), norm_coords: None },                     // 4
    KeyConfig { function: KeyFunction::HardKey(91), norm_coords: None },                     // 5
    KeyConfig { function: KeyFunction::HardKey(92), norm_coords: None },                     // 6
    KeyConfig { function: KeyFunction::HardKey(102), norm_coords: None },                    // THRU
    KeyConfig { function: KeyFunction::HardKey(107), norm_coords: Some((0.81849, 0.85333)) }, // UP
    // Row 9
    KeyConfig { function: KeyFunction::HardKey(57), norm_coords: Some((0.61838, 0.59810)) }, // UPDATE
    KeyConfig { function: KeyFunction::HardKey(105), norm_coords: Some((0.65896, 0.59810)) }, // CLEAR
    KeyConfig { function: KeyFunction::HardKey(87), norm_coords: None },                     // 1
    KeyConfig { function: KeyFunction::HardKey(88), norm_coords: None },                     // 2
    KeyConfig { function: KeyFunction::HardKey(89), norm_coords: None },                     // 3
    KeyConfig { function: KeyFunction::HardKey(97), norm_coords: None },                     // -
    KeyConfig { function: KeyFunction::HardKey(109), norm_coords: Some((0.77790, 0.91429)) }, // PREV
    KeyConfig { function: KeyFunction::HardKey(108), norm_coords: Some((0.81849, 0.91429)) }, // SET
    KeyConfig { function: KeyFunction::HardKey(110), norm_coords: Some((0.85795, 0.91429)) }, // NEXT
    KeyConfig { function: KeyFunction::HardKey(9), norm_coords: None },                      // DEF_PAUSE
    // Row 10
    KeyConfig { function: KeyFunction::HardKey(59), norm_coords: Some((0.61838, 0.67619)) }, // STORE
    KeyConfig { function: KeyFunction::HardKey(28), norm_coords: None },                     // X17
    KeyConfig { function: KeyFunction::HardKey(86), norm_coords: None },                     // 0
    KeyConfig { function: KeyFunction::HardKey(98), norm_coords: None },                     // .
    KeyConfig { function: KeyFunction::HardKey(103), norm_coords: None },                    // IF
    KeyConfig { function: KeyFunction::HardKey(104), norm_coords: None },                    // AT
    KeyConfig { function: KeyFunction::HardKey(111), norm_coords: Some((0.81849, 0.97238)) }, // DOWN
    KeyConfig { function: KeyFunction::HardKey(10), norm_coords: None },                     // DEF_GO-
    // Row 11
    KeyConfig { function: KeyFunction::HardKey(68), norm_coords: None }, // MA
    KeyConfig { function: KeyFunction::HardKey(106), norm_coords: None }, // PLEASE
    KeyConfig { function: KeyFunction::HardKey(11), norm_coords: None }, // DEF_GO+
    KeyConfig { function: KeyFunction::ExecButton(6), norm_coords: None }, // EXEC6
    KeyConfig { function: KeyFunction::ExecButton(7), norm_coords: None }, // EXEC7
    KeyConfig { function: KeyFunction::ExecButton(8), norm_coords: None }, // EXEC8
    KeyConfig { function: KeyFunction::ExecButton(9), norm_coords: None }, // EXEC9
];
