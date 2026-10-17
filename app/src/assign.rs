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

/// Readable name of a code, when there is one.
pub fn name(cod: u8) -> String {
    if (0x68..=0x73).contains(&cod) {
        format!("F{}", 13 + (cod - 0x68))
    } else {
        format!("0x{cod:02X}")
    }
}

/// `F13`..`F24` = `0x68`..`0x73`.
///
/// They are the good choice for a "free key": they exist in HID, Linux maps
/// them to `KEY_F13`..`KEY_F24`, and no software uses them out of the box
/// (`docs/28`).
pub fn free_codes() -> impl Iterator<Item = u8> {
    0x68u8..=0x73
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
