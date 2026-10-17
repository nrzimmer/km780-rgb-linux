//! Key assignment table: bytes 120-263 of the mode payload.
//!
//! One byte per LED slot, holding that key's function code (`docs/09` §5.1
//! and §5.2). The table lives **in the keyboard's memory**, so a key remapped
//! here stays remapped with no process running, survives a reboot, survives
//! replugging, and holds on another computer. Validated on the wire in
//! `docs/28`.
//!
//! This module mirrors `tools/km780assign.py`, **including the record file**:
//! both read and write the same `~/.config/km780/assignments.json`, with the
//! same `"mode/slot"` keys. A divergent format would make one tool's `restore`
//! blind to what the other stored.

use std::collections::BTreeMap;
use std::path::PathBuf;

/// First byte of the table inside the mode payload.
pub const OFF_FUNCTIONS: usize = 120;

/// The ranges of the function byte (`docs/09` §5.2), so the UI does not lie
/// about what a raw code will do.
pub fn range_name(cod: u8) -> &'static str {
    match cod {
        0x00 => "no function",
        0xA5..=0xAB => "multimedia",
        0xB4 => "macro",
        0xB8 => "text",
        0xC0..=0xC8 => "Windows shortcut",
        0xCE..=0xD5 => "mouse function",
        0x68..=0x73 => "F13..F24 \u{2014} the free-key range",
        0x04..=0xE7 => "HID usage (ordinary key)",
        _ => "outside the known ranges",
    }
}

/// An assignable function: code and name for the screen.
pub struct Function {
    pub code: u8,
    pub name: &'static str,
}

const fn f(code: u8, name: &'static str) -> Function {
    Function { code, name }
}

/// The ranges offered, in the order they appear in the combo box.
///
/// Macro (`0xB4`) and Text (`0xB8`) are **not** here, each for a different
/// reason. A macro needs content, which lives in `0x05` and in `0x0A` page
/// `0x09`, and the app does not manage either yet -- assigning `B4` would
/// point at a slot nobody maintains. Text never works without G.SKILL's app
/// running: the content does not reach the keyboard, only the name
/// (`docs/17` §5). Anyone who knows what they are doing reaches both through
/// the hex field.
pub static GROUPS: &[(&str, &[Function])] = &[
    ("Off", &[f(0x00, "No function")]),
    ("Free keys", F13_F24),
    ("Keyboard", KEYBOARD),
    ("Multimedia", MULTIMEDIA),
    ("Mouse", MOUSE),
    ("Windows", SHORTCUTS),
];

/// `0x68`-`0x73`. They exist in HID, Linux maps them to `KEY_F13`..`KEY_F24`
/// and no software uses them out of the box -- the natural "free key" space
/// (`docs/28`).
static F13_F24: &[Function] = &[
    f(0x68, "F13"), f(0x69, "F14"), f(0x6A, "F15"), f(0x6B, "F16"),
    f(0x6C, "F17"), f(0x6D, "F18"), f(0x6E, "F19"), f(0x6F, "F20"),
    f(0x70, "F21"), f(0x71, "F22"), f(0x72, "F23"), f(0x73, "F24"),
];

/// HID usage of each key, from the factory table in `docs/10`.
///
/// Assigning one of these makes the key **act as another** -- CapsLock
/// becoming Esc, for example. Generated from the same table `docs/protocol.html`
/// publishes, filtered to the real keyboard ranges (`0x04`-`0x65` and the
/// modifiers `0xE0`-`0xE7`); the high codes of the special keys are not HID
/// and stay out.
static KEYBOARD: &[Function] = &[
    f(0x04, "A"), f(0x05, "B"), f(0x06, "C"), f(0x07, "D"),
    f(0x08, "E"), f(0x09, "F"), f(0x0A, "G"), f(0x0B, "H"),
    f(0x0C, "I"), f(0x0D, "J"), f(0x0E, "K"), f(0x0F, "L"),
    f(0x10, "M"), f(0x11, "N"), f(0x12, "O"), f(0x13, "P"),
    f(0x14, "Q"), f(0x15, "R"), f(0x16, "S"), f(0x17, "T"),
    f(0x18, "U"), f(0x19, "V"), f(0x1A, "W"), f(0x1B, "X"),
    f(0x1C, "Y"), f(0x1D, "Z"), f(0x1E, "1"), f(0x1F, "2"),
    f(0x20, "3"), f(0x21, "4"), f(0x22, "5"), f(0x23, "6"),
    f(0x24, "7"), f(0x25, "8"), f(0x26, "9"), f(0x27, "0"),
    f(0x28, "Enter"), f(0x29, "Esc"), f(0x2A, "Backspace"), f(0x2B, "Tab"),
    f(0x2C, "Space"), f(0x2D, "-"), f(0x2E, "="), f(0x2F, "["),
    f(0x30, "]"), f(0x31, "\\"), f(0x32, "VK22"), f(0x33, ";"),
    f(0x34, "'"), f(0x35, "`"), f(0x36, ","), f(0x37, "."),
    f(0x38, "/"), f(0x39, "CapsLock"), f(0x3A, "F1"), f(0x3B, "F2"),
    f(0x3C, "F3"), f(0x3D, "F4"), f(0x3E, "F5"), f(0x3F, "F6"),
    f(0x40, "F7"), f(0x41, "F8"), f(0x42, "F9"), f(0x43, "F10"),
    f(0x44, "F11"), f(0x45, "F12"), f(0x46, "PrtSc"), f(0x47, "ScrLk"),
    f(0x48, "Pause"), f(0x49, "Insert"), f(0x4A, "Home"), f(0x4B, "PgUp"),
    f(0x4C, "Delete"), f(0x4D, "End"), f(0x4E, "PgDn"), f(0x4F, "Right"),
    f(0x50, "Left"), f(0x51, "Down"), f(0x52, "Up"), f(0x53, "NumLock"),
    f(0x54, "Num/"), f(0x55, "Num*"), f(0x56, "Num-"), f(0x57, "Num+"),
    f(0x58, "NumSep"), f(0x59, "Num1"), f(0x5A, "Num2"), f(0x5B, "Num3"),
    f(0x5C, "Num4"), f(0x5D, "Num5"), f(0x5E, "Num6"), f(0x5F, "Num7"),
    f(0x60, "Num8"), f(0x61, "Num9"), f(0x62, "Num0"), f(0x63, "Num."),
    f(0x64, "|"), f(0x65, "Menu"), f(0xE0, "LCtrl"), f(0xE1, "LShift"),
    f(0xE2, "LAlt"), f(0xE3, "LWin"), f(0xE4, "RCtrl"), f(0xE5, "RShift"),
    f(0xE6, "RAlt"), f(0xE7, "RWin"),
];

/// `docs/17` §3. The first five match what the factory table already carries
/// on the physical media keys -- an independent check.
static MULTIMEDIA: &[Function] = &[
    f(0xA5, "Stop"),
    f(0xA6, "Previous track"),
    f(0xA7, "Play / Pause"),
    f(0xA8, "Next track"),
    f(0xA9, "Mute"),
    f(0xAA, "Volume down"),
    f(0xAB, "Volume up"),
];

/// `docs/17` §2. The code order is by category, not the UI list's order.
static MOUSE: &[Function] = &[
    f(0xCE, "Left click"),
    f(0xCF, "Right click"),
    f(0xD0, "Scroll click"),
    f(0xD1, "Scroll down"),
    f(0xD2, "Scroll up"),
    f(0xD3, "Backward"),
    f(0xD4, "Forward"),
    f(0xD5, "Double click"),
];

/// `docs/17` §4. Nine codes, **contiguous**.
///
/// `C3` and `C4` were taken for a hole because the capture walked the seven
/// items the official dialog offers, and the dialog does not list these two.
/// They exist: measured on hardware on 2026-10-17, they come out as
/// `Super`+`Tab` and `Alt`+`Tab` (`docs/17` §4.1). The second case of a
/// feature the firmware has and the official app hides -- the first is effect
/// `0x06`.
static SHORTCUTS: &[Function] = &[
    f(0xC0, "Launch Task Manager"),
    f(0xC1, "Open Start menu"),
    f(0xC2, "Show desktop"),
    f(0xC3, "Task view (Super+Tab)"),
    f(0xC4, "Switch window (Alt+Tab)"),
    f(0xC5, "Close active window"),
    f(0xC6, "Cut"),
    f(0xC7, "Copy"),
    f(0xC8, "Paste"),
];

/// Known name of a code, from any range.
pub fn code_name(code: u8) -> Option<&'static str> {
    GROUPS
        .iter()
        .flat_map(|(_, fs)| fs.iter())
        .find(|f| f.code == code)
        .map(|f| f.name)
}

/// Readable name of a code, when there is one.
pub fn name(cod: u8) -> String {
    match code_name(cod) {
        Some(n) => n.to_string(),
        None => format!("0x{cod:02X}"),
    }
}


/// Slots whose function differs between two mode payloads.
///
/// Pure on purpose, so it can be tested without hardware: it is the
/// "what changed" decision that the GUI's `Apply` and the CLI's `--function`
/// both make.
pub fn remapped(before: &[u8], after: &[u8]) -> Vec<usize> {
    if before.len() != after.len() {
        return Vec::new();
    }
    (0..crate::slots::NSLOTS)
        .filter(|&i| {
            let o = OFF_FUNCTIONS + i;
            o < before.len() && before[o] != after[o]
        })
        .collect()
}

/// Carries the function bytes that changed over to the **other** modes,
/// saving each one's previous value.
///
/// The assignment table is per mode, and the M1/M2/M3 keys switch the mode
/// inside the keyboard without telling the host. Remapping only in the active
/// mode makes the key stop working when the user switches profile -- the first
/// of the two traps in `docs/28`.
///
/// It carries **only the bytes that changed**, never the whole payload of the
/// active mode: that would drag its color, brightness and effect over to the
/// other profiles.
///
/// This function is the single path: the GUI calls it on `Apply` and so does
/// `--function` on the CLI. What one validated on the wire holds for the other.
pub fn propagate(
    kb: &crate::proto::Keyboard,
    active_mode: u8,
    before: &[u8],
    after: &[u8],
) -> Result<Vec<usize>, String> {
    let changed = remapped(before, after);
    if changed.is_empty() {
        return Ok(changed);
    }
    let mut record = Record::load();
    for &slot in &changed {
        record.remember(active_mode, slot, before[OFF_FUNCTIONS + slot]);
    }
    for other in 1..=3u8 {
        if other == active_mode {
            continue;
        }
        let mut d = kb.read_mode(other)?;
        for &slot in &changed {
            let o = OFF_FUNCTIONS + slot;
            record.remember(other, slot, d[o]);
            d[o] = after[o];
        }
        kb.write_mode(other, &d)?;
    }
    record.save()?;
    Ok(changed)
}

fn file_path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
    Some(base.join("km780").join("assignments.json"))
}

/// The record of previous values, keyed by `"mode/slot"`.
#[derive(Default)]
pub struct Record {
    map: BTreeMap<String, u8>,
}

impl Record {
    pub fn load() -> Record {
        let mut map = BTreeMap::new();
        if let Some(p) = file_path() {
            if let Ok(text) = std::fs::read_to_string(&p) {
                if let Ok(serde_json::Value::Object(o)) = serde_json::from_str(&text) {
                    for (k, v) in o {
                        if let Some(n) = v.as_u64() {
                            if n <= 255 {
                                map.insert(k, n as u8);
                            }
                        }
                    }
                }
            }
        }
        Record { map }
    }

    pub fn save(&self) -> Result<(), String> {
        let p = file_path().ok_or("missing HOME or XDG_CONFIG_HOME")?;
        if let Some(d) = p.parent() {
            std::fs::create_dir_all(d).map_err(|e| format!("{}: {e}", d.display()))?;
        }
        let o: serde_json::Map<String, serde_json::Value> = self
            .map
            .iter()
            .map(|(k, v)| (k.clone(), serde_json::Value::from(*v)))
            .collect();
        let text = serde_json::to_string_pretty(&serde_json::Value::Object(o))
            .map_err(|e| e.to_string())?;
        std::fs::write(&p, text).map_err(|e| format!("{}: {e}", p.display()))
    }

    pub fn previous(&self, mode: u8, slot: usize) -> Option<u8> {
        self.map.get(&format!("{mode}/{slot}")).copied()
    }

    /// Stores the previous value **only if there is not one yet**.
    ///
    /// Not a detail: reapplying the same remap would overwrite the original
    /// with the new value and destroy the undo. It was a real bug in
    /// `km780assign.py`, fixed with the same `setdefault` (`docs/28`).
    pub fn remember(&mut self, mode: u8, slot: usize, previous_value: u8) {
        self.map.entry(format!("{mode}/{slot}")).or_insert(previous_value);
    }

    pub fn path() -> String {
        file_path().map(|p| p.display().to_string()).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proto::PKT;

    #[test]
    fn only_function_bytes_count() {
        let before = vec![0u8; PKT];
        let mut after = before.clone();
        // byte 11 is the brightness, not the function table: it must not become a remap.
        after[crate::proto::OFF_BRIGHTNESS] = 4;
        assert!(remapped(&before, &after).is_empty());

        after[OFF_FUNCTIONS + 44] = 0x68; // Timer -> F13
        assert_eq!(remapped(&before, &after), vec![44]);
    }

    #[test]
    fn remember_keeps_the_first_value() {
        // The real bug from docs/28: reapplying the same remap recorded the
        // new value as the "previous" one and destroyed the undo.
        let mut r = Record::default();
        r.remember(2, 44, 0xBA);
        r.remember(2, 44, 0x68);
        assert_eq!(r.previous(2, 44), Some(0xBA));
    }
}
