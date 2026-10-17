//! Physical layout of the keyboard, for drawing the clickable matrix.
//!
//! **Written by hand, on purpose.** `physical-matrix.json` is the *electrical*
//! scan matrix, and `docs/25` warns that "the matrix wiring does not follow
//! physical adjacency" -- M3 comes out at (2,6), next to F3. Drawing the
//! keyboard from it would give a scrambled keyboard.
//!
//! The coordinates are in key units (1u = one ordinary key), with the origin
//! at the top-left corner of the main block. Adjusting here is just a matter
//! of numbers; nothing else in the app depends on them.
//!
//! The names have to be exactly those in `slots::SLOTS` -- the test at the end
//! of this file guarantees that.

/// One drawn key: the slot name, the top-left corner, width, height.
pub struct Key {
    pub name: &'static str,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

const fn k(name: &'static str, x: f32, y: f32) -> Key {
    Key { name, x, y, w: 1.0, h: 1.0 }
}
const fn kw(name: &'static str, x: f32, y: f32, w: f32) -> Key {
    Key { name, x, y, w, h: 1.0 }
}
const fn kh(name: &'static str, x: f32, y: f32, h: f32) -> Key {
    Key { name, x, y, w: 1.0, h }
}

// Rows, in key units.
const R_TOP: f32 = -1.15; // the WinLock/Brightness/Timer and multimedia strip
const R_FN: f32 = 0.0;
const R_NUM: f32 = 1.25;
const R_Q: f32 = 2.25;
const R_A: f32 = 3.25;
const R_Z: f32 = 4.25;
const R_CTRL: f32 = 5.25;

// Reference columns.
const X_MAIN: f32 = 2.4; // start of the main block
const X_NAV: f32 = X_MAIN + 15.25; // PrtSc / Insert / arrows
const X_PAD: f32 = X_MAIN + 18.5; // numeric keypad

pub static KEYS: &[Key] = &[
    // --- column of G keys, on the left side -----------------------------
    k("G1", 0.0, R_FN + 0.1),
    k("G2", 0.0, R_FN + 1.1),
    k("G3", 0.0, R_FN + 2.1),
    k("G4", 0.0, R_FN + 3.1),
    k("G5", 0.0, R_FN + 4.1),
    k("G6", 0.0, R_FN + 5.1),
    // --- modes and macro recorder ---------------------------------------
    k("MR", 1.15, R_FN + 0.1),
    k("M1", 1.15, R_FN + 1.1),
    k("M2", 1.15, R_FN + 2.1),
    k("M3", 1.15, R_FN + 3.1),
    // --- top strip -------------------------------------------------------
    k("WinLock", X_NAV - 1.0, R_TOP),
    k("Brightness", X_NAV, R_TOP),
    k("Timer", X_NAV + 1.0, R_TOP),
    k("Mute", X_NAV + 2.0, R_TOP),
    k("Stop", X_PAD, R_TOP),
    k("Prev", X_PAD + 1.0, R_TOP),
    k("Play", X_PAD + 2.0, R_TOP),
    k("Next", X_PAD + 3.0, R_TOP),
    // --- function row -----------------------------------------------------
    k("Esc", X_MAIN, R_FN),
    k("F1", X_MAIN + 2.0, R_FN),
    k("F2", X_MAIN + 3.0, R_FN),
    k("F3", X_MAIN + 4.0, R_FN),
    k("F4", X_MAIN + 5.0, R_FN),
    k("F5", X_MAIN + 6.5, R_FN),
    k("F6", X_MAIN + 7.5, R_FN),
    k("F7", X_MAIN + 8.5, R_FN),
    k("F8", X_MAIN + 9.5, R_FN),
    k("F9", X_MAIN + 11.0, R_FN),
    k("F10", X_MAIN + 12.0, R_FN),
    k("F11", X_MAIN + 13.0, R_FN),
    k("F12", X_MAIN + 14.0, R_FN),
    k("PrtSc", X_NAV, R_FN),
    k("ScrLk", X_NAV + 1.0, R_FN),
    k("Pause", X_NAV + 2.0, R_FN),
    // --- number row -------------------------------------------------------
    k("`", X_MAIN, R_NUM),
    k("1", X_MAIN + 1.0, R_NUM),
    k("2", X_MAIN + 2.0, R_NUM),
    k("3", X_MAIN + 3.0, R_NUM),
    k("4", X_MAIN + 4.0, R_NUM),
    k("5", X_MAIN + 5.0, R_NUM),
    k("6", X_MAIN + 6.0, R_NUM),
    k("7", X_MAIN + 7.0, R_NUM),
    k("8", X_MAIN + 8.0, R_NUM),
    k("9", X_MAIN + 9.0, R_NUM),
    k("0", X_MAIN + 10.0, R_NUM),
    k("-", X_MAIN + 11.0, R_NUM),
    k("=", X_MAIN + 12.0, R_NUM),
    kw("Backspace", X_MAIN + 13.0, R_NUM, 2.0),
    k("Insert", X_NAV, R_NUM),
    k("Home", X_NAV + 1.0, R_NUM),
    k("PgUp", X_NAV + 2.0, R_NUM),
    // --- Q row -------------------------------------------------------------
    kw("Tab", X_MAIN, R_Q, 1.5),
    k("Q", X_MAIN + 1.5, R_Q),
    k("W", X_MAIN + 2.5, R_Q),
    k("E", X_MAIN + 3.5, R_Q),
    k("R", X_MAIN + 4.5, R_Q),
    k("T", X_MAIN + 5.5, R_Q),
    k("Y", X_MAIN + 6.5, R_Q),
    k("U", X_MAIN + 7.5, R_Q),
    k("I", X_MAIN + 8.5, R_Q),
    k("O", X_MAIN + 9.5, R_Q),
    k("P", X_MAIN + 10.5, R_Q),
    k("[", X_MAIN + 11.5, R_Q),
    k("]", X_MAIN + 12.5, R_Q),
    kw("\\", X_MAIN + 13.5, R_Q, 1.5),
    k("Delete", X_NAV, R_Q),
    k("End", X_NAV + 1.0, R_Q),
    k("PgDn", X_NAV + 2.0, R_Q),
    // --- A row -------------------------------------------------------------
    kw("CapsLock", X_MAIN, R_A, 1.75),
    k("A", X_MAIN + 1.75, R_A),
    k("S", X_MAIN + 2.75, R_A),
    k("D", X_MAIN + 3.75, R_A),
    k("F", X_MAIN + 4.75, R_A),
    k("G", X_MAIN + 5.75, R_A),
    k("H", X_MAIN + 6.75, R_A),
    k("J", X_MAIN + 7.75, R_A),
    k("K", X_MAIN + 8.75, R_A),
    k("L", X_MAIN + 9.75, R_A),
    k(";", X_MAIN + 10.75, R_A),
    k("'", X_MAIN + 11.75, R_A),
    kw("Enter", X_MAIN + 12.75, R_A, 2.25),
    // --- Z row -------------------------------------------------------------
    kw("LShift", X_MAIN, R_Z, 2.25),
    k("Z", X_MAIN + 2.25, R_Z),
    k("X", X_MAIN + 3.25, R_Z),
    k("C", X_MAIN + 4.25, R_Z),
    k("V", X_MAIN + 5.25, R_Z),
    k("B", X_MAIN + 6.25, R_Z),
    k("N", X_MAIN + 7.25, R_Z),
    k("M", X_MAIN + 8.25, R_Z),
    k(",", X_MAIN + 9.25, R_Z),
    k(".", X_MAIN + 10.25, R_Z),
    k("/", X_MAIN + 11.25, R_Z),
    kw("RShift", X_MAIN + 12.25, R_Z, 2.75),
    k("Up", X_NAV + 1.0, R_Z),
    // --- bottom row --------------------------------------------------------
    kw("LCtrl", X_MAIN, R_CTRL, 1.25),
    kw("LWin", X_MAIN + 1.25, R_CTRL, 1.25),
    kw("LAlt", X_MAIN + 2.5, R_CTRL, 1.25),
    kw("Space", X_MAIN + 3.75, R_CTRL, 6.25),
    kw("RAlt", X_MAIN + 10.0, R_CTRL, 1.25),
    kw("RWin", X_MAIN + 11.25, R_CTRL, 1.25),
    kw("Menu", X_MAIN + 12.5, R_CTRL, 1.25),
    kw("RCtrl", X_MAIN + 13.75, R_CTRL, 1.25),
    k("Left", X_NAV, R_CTRL),
    k("Down", X_NAV + 1.0, R_CTRL),
    k("Right", X_NAV + 2.0, R_CTRL),
    // --- numeric keypad -----------------------------------------------------
    k("NumLock", X_PAD, R_NUM),
    k("Num/", X_PAD + 1.0, R_NUM),
    k("Num*", X_PAD + 2.0, R_NUM),
    k("Num-", X_PAD + 3.0, R_NUM),
    k("Num7", X_PAD, R_Q),
    k("Num8", X_PAD + 1.0, R_Q),
    k("Num9", X_PAD + 2.0, R_Q),
    kh("Num+", X_PAD + 3.0, R_Q, 2.0),
    k("Num4", X_PAD, R_A),
    k("Num5", X_PAD + 1.0, R_A),
    k("Num6", X_PAD + 2.0, R_A),
    k("Num1", X_PAD, R_Z),
    k("Num2", X_PAD + 1.0, R_Z),
    k("Num3", X_PAD + 2.0, R_Z),
    // The keypad Enter: `docs/25` calls it NumEnter, and the slot list has a
    // `NumSep` with no confirmed position. They are not obviously the same key,
    // and guessing would put a color on the wrong one -- it stays among the
    // "other slots".
    kw("Num0", X_PAD, R_CTRL, 2.0),
    k("Num.", X_PAD + 2.0, R_CTRL),
];

/// Width and height the drawing takes up, in key units.
pub fn extent() -> (f32, f32) {
    let mut x1 = 0.0f32;
    let mut y1 = 0.0f32;
    for t in KEYS {
        x1 = x1.max(t.x + t.w);
        y1 = y1.max(t.y + t.h);
    }
    (x1, y1 - R_TOP)
}

/// Vertical offset to add so that nothing ends up with a negative y.
pub const Y_OFFSET: f32 = -R_TOP;

#[cfg(test)]
mod tests {
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
            assert!(!seen.contains(&t.name), "{:?} shows up twice", t.name);
            seen.push(t.name);
        }
    }
}
