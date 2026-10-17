//! Physical layout of the keyboard, **from the manufacturer itself**.
//!
//! The first 110 keys are generated from
//! `external/gds/mx-v0.59/payload/app/KeyboardButtons.xml`, the file the
//! official app uses to draw its own keyboard: each `<POS>` carries x, y,
//! width and height in pixels. It is not an estimate nor a photo measured by
//! eye -- it is the position G.SKILL uses.
//!
//! The remaining 12 -- MR, M1-M3, the WinLock/Brightness/Timer trio and the
//! five multimedia ones -- **are not in that XML**, because the official app
//! does not let you reassign them (`<KEY_NUM>104</KEY_NUM>`, plus the 6 G
//! keys). Those were measured on the photo in
//! `Keyboard Wireframe_01232015v3.pdf`, from the same package, and converted
//! to the same coordinate system using the neighbouring keys as reference.
//! They are the only approximate ones, and even so anchored in a measurement,
//! not in memory.
//!
//! **Independent check:** 110 + 12 = 122 keys, and 122 is exactly what
//! `docs/25` counted by sweeping the electrical matrix on the hardware. Two
//! sources that never talk to each other arriving at the same number.
//!
//! The unit is the pixel of the manufacturer's drawing; the UI scales by a
//! single factor. The test at the end validates every name against
//! `slots::SLOTS`.

/// One drawn key: the slot name and the rectangle in manufacturer pixels.
pub struct Key {
    pub name: &'static str,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

const fn k(name: &'static str, x: f32, y: f32, w: f32, h: f32) -> Key {
    Key { name, x, y, w, h }
}

pub static KEYS: &[Key] = &[
    k("G1", 68.0, 161.0, 21.0, 21.0),
    k("G2", 68.0, 185.0, 21.0, 21.0),
    k("G3", 68.0, 207.0, 21.0, 21.0),
    k("G4", 68.0, 229.0, 21.0, 21.0),
    k("G5", 68.0, 251.0, 21.0, 21.0),
    k("G6", 68.0, 273.0, 21.0, 21.0),
    k("Esc", 97.0, 161.0, 19.0, 20.0),
    k("F1", 134.0, 161.0, 19.0, 20.0),
    k("F2", 155.0, 161.0, 19.0, 20.0),
    k("F3", 178.0, 161.0, 19.0, 20.0),
    k("F4", 200.0, 161.0, 19.0, 20.0),
    k("F5", 236.0, 161.0, 19.0, 20.0),
    k("F6", 258.0, 161.0, 19.0, 20.0),
    k("F7", 280.0, 161.0, 19.0, 20.0),
    k("F8", 302.0, 161.0, 19.0, 20.0),
    k("F9", 339.0, 161.0, 19.0, 20.0),
    k("F10", 361.0, 161.0, 19.0, 20.0),
    k("F11", 383.0, 161.0, 19.0, 20.0),
    k("F12", 405.0, 161.0, 19.0, 20.0),
    k("PrtSc", 432.0, 161.0, 19.0, 20.0),
    k("ScrLk", 454.0, 161.0, 19.0, 20.0),
    k("Pause", 478.0, 161.0, 19.0, 20.0),
    k("`", 97.0, 185.0, 19.0, 20.0),
    k("1", 119.0, 185.0, 19.0, 20.0),
    k("2", 141.0, 185.0, 19.0, 20.0),
    k("3", 163.0, 185.0, 19.0, 20.0),
    k("4", 185.0, 185.0, 19.0, 20.0),
    k("5", 207.0, 185.0, 19.0, 20.0),
    k("6", 229.0, 185.0, 19.0, 20.0),
    k("7", 251.0, 185.0, 19.0, 20.0),
    k("8", 273.0, 185.0, 19.0, 20.0),
    k("9", 295.0, 185.0, 19.0, 20.0),
    k("0", 317.0, 185.0, 19.0, 20.0),
    k("-", 339.0, 185.0, 19.0, 20.0),
    k("=", 361.0, 185.0, 19.0, 20.0),
    k("Backspace", 383.0, 185.0, 41.0, 20.0),
    k("Insert", 432.0, 185.0, 19.0, 20.0),
    k("Home", 454.0, 185.0, 19.0, 20.0),
    k("PgUp", 476.0, 185.0, 19.0, 20.0),
    k("NumLock", 504.0, 185.0, 19.0, 20.0),
    k("Num/", 526.0, 185.0, 19.0, 20.0),
    k("Num*", 548.0, 185.0, 19.0, 20.0),
    k("Num-", 570.0, 185.0, 19.0, 20.0),
    k("Tab", 97.0, 207.0, 30.0, 20.0),
    k("Q", 130.0, 207.0, 19.0, 20.0),
    k("W", 152.0, 207.0, 19.0, 20.0),
    k("E", 174.0, 207.0, 19.0, 20.0),
    k("R", 196.0, 207.0, 19.0, 20.0),
    k("T", 218.0, 207.0, 19.0, 20.0),
    k("Y", 240.0, 207.0, 19.0, 20.0),
    k("U", 262.0, 207.0, 19.0, 20.0),
    k("I", 284.0, 207.0, 19.0, 20.0),
    k("O", 306.0, 207.0, 19.0, 20.0),
    k("P", 328.0, 207.0, 19.0, 20.0),
    k("[", 350.0, 207.0, 19.0, 20.0),
    k("]", 372.0, 207.0, 19.0, 20.0),
    k("\\", 394.0, 207.0, 30.0, 20.0),
    k("Delete", 432.0, 207.0, 19.0, 20.0),
    k("End", 454.0, 207.0, 19.0, 20.0),
    k("PgDn", 476.0, 207.0, 19.0, 20.0),
    k("Num7", 504.0, 207.0, 19.0, 20.0),
    k("Num8", 526.0, 207.0, 19.0, 20.0),
    k("Num9", 548.0, 207.0, 19.0, 20.0),
    k("Num+", 570.0, 207.0, 19.0, 42.0),
    k("CapsLock", 97.0, 229.0, 35.0, 20.0),
    k("A", 135.0, 229.0, 19.0, 20.0),
    k("S", 157.0, 229.0, 19.0, 20.0),
    k("D", 179.0, 229.0, 19.0, 20.0),
    k("F", 201.0, 229.0, 19.0, 20.0),
    k("G", 223.0, 229.0, 19.0, 20.0),
    k("H", 245.0, 229.0, 19.0, 20.0),
    k("J", 267.0, 229.0, 19.0, 20.0),
    k("K", 289.0, 229.0, 19.0, 20.0),
    k("L", 311.0, 229.0, 19.0, 20.0),
    k(";", 333.0, 229.0, 19.0, 20.0),
    k("'", 355.0, 229.0, 19.0, 20.0),
    k("Enter", 378.0, 229.0, 46.0, 20.0),
    k("Num4", 504.0, 229.0, 19.0, 20.0),
    k("Num5", 526.0, 229.0, 19.0, 20.0),
    k("Num6", 548.0, 229.0, 19.0, 20.0),
    k("LShift", 97.0, 251.0, 46.0, 20.0),
    k("Z", 146.0, 251.0, 19.0, 20.0),
    k("X", 168.0, 251.0, 19.0, 20.0),
    k("C", 190.0, 251.0, 19.0, 20.0),
    k("V", 212.0, 251.0, 19.0, 20.0),
    k("B", 234.0, 251.0, 19.0, 20.0),
    k("N", 256.0, 251.0, 19.0, 20.0),
    k("M", 278.0, 251.0, 19.0, 20.0),
    k(",", 300.0, 251.0, 19.0, 20.0),
    k(".", 322.0, 251.0, 19.0, 20.0),
    k("/", 344.0, 251.0, 19.0, 20.0),
    k("RShift", 366.0, 251.0, 57.0, 20.0),
    k("Up", 454.0, 251.0, 19.0, 20.0),
    k("Num1", 504.0, 251.0, 19.0, 20.0),
    k("Num2", 526.0, 251.0, 19.0, 20.0),
    k("Num3", 548.0, 251.0, 19.0, 20.0),
    k("NumSep", 570.0, 251.0, 19.0, 42.0),
    k("LCtrl", 97.0, 273.0, 30.0, 20.0),
    k("LWin", 130.0, 273.0, 30.0, 20.0),
    k("LAlt", 163.0, 273.0, 30.0, 20.0),
    k("Space", 195.0, 273.0, 117.0, 20.0),
    k("RAlt", 317.0, 273.0, 24.0, 20.0),
    k("RWin", 344.0, 273.0, 24.0, 20.0),
    k("Menu", 371.0, 273.0, 24.0, 20.0),
    k("RCtrl", 400.0, 273.0, 24.0, 20.0),
    k("Left", 432.0, 273.0, 19.0, 20.0),
    k("Down", 454.0, 273.0, 19.0, 20.0),
    k("Right", 476.0, 273.0, 19.0, 20.0),
    k("Num0", 504.0, 273.0, 41.0, 20.0),
    k("Num.", 548.0, 273.0, 19.0, 20.0),
    k("MR", 73.0, 144.0, 19.0, 13.0),
    k("M1", 95.0, 144.0, 19.0, 13.0),
    k("M2", 114.0, 144.0, 19.0, 13.0),
    k("M3", 133.0, 144.0, 19.0, 13.0),
    k("WinLock", 162.0, 144.0, 19.0, 13.0),
    k("Brightness", 181.0, 144.0, 19.0, 13.0),
    k("Timer", 199.0, 144.0, 19.0, 13.0),
    k("Stop", 439.0, 144.0, 21.0, 13.0),
    k("Prev", 462.0, 144.0, 21.0, 13.0),
    k("Play", 486.0, 144.0, 21.0, 13.0),
    k("Next", 509.0, 144.0, 21.0, 13.0),
    k("Mute", 532.0, 144.0, 21.0, 13.0),
];

/// Top-left corner and extent of the drawing, in the manufacturer's pixels.
pub fn extent() -> (f32, f32, f32, f32) {
    let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, 0.0f32, 0.0f32);
    for t in KEYS {
        x0 = x0.min(t.x);
        y0 = y0.min(t.y);
        x1 = x1.max(t.x + t.w);
        y1 = y1.max(t.y + t.h);
    }
    (x0, y0, x1 - x0, y1 - y0)
}

#[cfg(test)]
mod teste {
    use super::*;
    use crate::slots;

    #[test]
    fn every_layout_name_is_a_slot() {
        for t in KEYS {
            assert!(
                slots::slot_of(t.name).is_some(),
                "the layout names {:?}, which is not a slot name",
                t.name
            );
        }
    }

    #[test]
    fn no_duplicate_keys() {
        let mut seen = Vec::new();
        for t in KEYS {
            assert!(!seen.contains(&t.name), "{:?} appears twice", t.name);
            seen.push(t.name);
        }
    }

    /// 122 physical keys, which is what docs/25 counted in the electrical
    /// matrix. If this test fails, the layout and the matrix disagree and one
    /// of the two is wrong.
    #[test]
    fn is_122_physical_keys() {
        assert_eq!(KEYS.len(), 122);
    }
}
