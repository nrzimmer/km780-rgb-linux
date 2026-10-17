//! Names of the 132 slots in the LED payload, ported from `tools/km780lib.py`
//! (generated from that list, not retyped). Index = slot in the 144-byte
//! payload of command 0x09/0x89. The last 12 slots have no known name.

/// Addressable slots in the payload (132 named out of NSLOTS=144).
pub const NSLOTS: usize = 144;

pub static SLOTS: [&str; 132] = [
    "G1"          , "G2"          , "G3"          , "G4",
    "G5"          , "G6"          , "MR"          , "M1",
    "M2"          , "Esc"         , "`"           , "Tab",
    "CapsLock"    , "LShift"      , "LCtrl"       , "1",
    "\\|"         , "LWin"        , "F1"          , "2",
    "Q"           , "A"           , "Z"           , "LAlt",
    "M3"          , "W"           , "X"           , "F2",
    "3"           , "E"           , "S"           , "C",
    "VK25"        , "F3"          , "D"           , "INTL8b",
    "F4"          , "4"           , "R"           , "F",
    "V"           , "5"           , "WinLock"     , "Brightness",
    "Timer"       , "F5"          , "6"           , "T",
    "G"           , "B"           , "Space"       , "7",
    "Y"           , "H"           , "F6"          , "8",
    "U"           , "J"           , "N"           , "F7",
    "I"           , "K"           , "M"           , "F8",
    "9"           , "O"           , "L"           , ",",
    "VK61"        , "F9"          , "0"           , "VK28",
    "["           , "-"           , "P"           , ";",
    "."           , "RAlt"        , "/"           , "RWin",
    "INTL88"      , "F11"         , "="           , "F10",
    "'"           , "INTL87"      , "Menu"        , "]",
    "VK22"        , "RCtrl"       , "F12"         , "INTL89",
    "\\"          , "Enter"       , "RShift"      , "Left",
    "Backspace"   , "Up"          , "Down"        , "PrtSc",
    "Insert"      , "Delete"      , "ScrLk"       , "Home",
    "End"         , "Stop"        , "Prev"        , "Play",
    "Pause"       , "PgUp"        , "PgDn"        , "Num4",
    "Num1"        , "NumLock"     , "Num7"        , "Mute",
    "Next"        , "Num*"        , "Num/"        , "Num8",
    "Num5"        , "Num2"        , "Num0"        , "Num9",
    "Num6"        , "Right"       , "NumSep"      , "Num-",
    "Num+"        , "INTL85"      , "Num3"        , "Num.",
];

/// Slots the matrix addresses but that have no physical key in the US/ABNT
/// layout -- the five with a HID code in the international range 0x85-0x8b.
/// See `docs/23` §6: "dark" does not mean "no key".
pub static NO_PHYSICAL_KEY: [usize; 5] = [35, 80, 85, 91, 129];

/// Name to show on screen.
///
/// `SLOTS` is the **canonical** table, shared with `docs/10` and with the
/// Python tools -- renaming there would break both ends. The only entry that
/// differs here is the brightness key, shortened to fit the key cap, so this
/// is a single display exception and not a second table.
pub fn display_name(name: &str) -> &str {
    match name {
        "Brightness" => "Bright",
        other => other,
    }
}

/// Slot of a key by name, as in `docs/10-key-map.md`, **or by number**.
///
/// The number matters: 22 of the 144 slots have no key in the drawing (10
/// named without a physical key in the US/ABNT layout, and the 132-143 tail
/// that has no name at all). The GUI does not show them, but they exist in the
/// payload and on this keyboard some are lit -- the number is how to reach
/// them from the CLI. See `docs/30`.
pub fn slot_of(name: &str) -> Option<usize> {
    let t = name.trim();
    if let Ok(n) = t.parse::<usize>() {
        return (n < NSLOTS).then_some(n);
    }
    SLOTS.iter().position(|s| s.eq_ignore_ascii_case(t))
}
