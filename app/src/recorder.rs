//! Macro recording from the window: `egui` events -> HID usage.
//!
//! ## Why record in the window, and not by listening to X11
//!
//! `tools/listen_x11.py` listens to the whole keyboard, from any window.
//! That is right for measuring the keyboard and **wrong** for recording a
//! macro: besides catching what the user types elsewhere, it only exists on
//! Linux with X11. Through `egui`'s events the recording is the same on
//! Windows, which is the project's declared target (`docs/29`).
//!
//! The price is that the window has to be focused and that keys `egui` itself
//! consumes -- Tab moves the focus, Space activates the button -- have to be
//! taken away from it while recording. That is what `Recorder::active` does.
//!
//! ## HID usage, not `egui::Key`
//!
//! The keyboard stores HID usage, which is **key position**, and `egui::Key`
//! is a **logical** key, after the user's map: on an ABNT2 the key next to L
//! sends `Ccedilla`, which is no HID at all. That is why `physical_key` comes
//! first and `key` only as a fallback (`Event::Key` carries both).
//!
//! ## Modifiers
//!
//! `egui` does **not** send a key event for Ctrl, Shift, Alt and Super -- they
//! arrive only as each event's `modifiers` field. So their press and release
//! are synthesized by comparing the state with the previous frame's. This
//! matches the wire format, where a modifier is an ordinary key (`0xE0` goes
//! down and up like any other, `docs/21` §2).
//!
//! An honest limitation: `egui` does not tell left Ctrl from right, only
//! "ctrl". We always record the left one. Anyone who needs the right one edits
//! the event afterwards, or records with the MR key on the keyboard itself.

use eframe::egui;

use crate::macros::{Event, MAX_EVENTS};

/// State of the recording in progress.
#[derive(Default)]
pub struct Recorder {
    pub active: bool,
    /// `(press, usage, ms since the recording started)`.
    raw: Vec<(bool, u8, u64)>,
    /// The moment of `Record`, on `egui`'s clock.
    started: f64,
    /// Modifiers in the previous frame, for synthesizing press/release.
    modifiers: egui::Modifiers,
    /// Keys whose press was recorded, so auto-repeat is not recorded.
    down: Vec<u8>,
}

impl Recorder {
    pub fn start(&mut self, now: f64) {
        self.active = true;
        self.raw.clear();
        self.started = now;
        self.modifiers = egui::Modifiers::default();
        self.down.clear();
    }

    pub fn stop(&mut self) {
        self.active = false;
        self.down.clear();
    }

    /// How many raw events have come in. Used only by the tests; the screen
    /// reads the finished list through `events`.
    #[cfg(test)]
    fn len(&self) -> usize {
        self.raw.len()
    }

    pub fn full(&self) -> bool {
        self.raw.len() >= MAX_EVENTS
    }

    /// The recorded events, with the delays already computed.
    pub fn events(&self) -> Vec<Event> {
        crate::macros::from_recording(&self.raw)
    }

    /// Collects what happened in this frame. Called once per frame while
    /// `active`.
    ///
    /// Returns `true` if anything was recorded -- the UI uses that to request
    /// a repaint and keep the list alive on screen.
    pub fn collect(&mut self, ui: &egui::Ui) -> bool {
        let mut any = false;
        let (events, final_modifiers, now) =
            ui.input(|i| (i.events.clone(), i.modifiers, i.time));
        let at = ((now - self.started) * 1000.0).max(0.0) as u64;

        for e in &events {
            // Every event carries the modifiers **of its own moment**, and it
            // is by those that the state is synchronized -- not by the state
            // at the end of the frame.
            //
            // The difference shows up in Shift+Z. When the Z press and the
            // Shift release land in the same frame, synchronizing by the final
            // state first recorded "release Shift" **before** "press Z", and
            // the macro would type a lowercase z. Measured on screen.
            if let Some(m) = event_modifiers(e) {
                any |= self.sync_modifiers(m, at);
            }

            // Ctrl+C, Ctrl+X and Ctrl+V never arrive as a key: `egui` turns
            // them into `Copy`/`Cut`/`Paste` first. Without this, recording a
            // Ctrl+C kept the Ctrl and lost the C -- measured on screen.
            if let Some(usage) = match e {
                egui::Event::Copy => Some(0x06u8),  // C
                egui::Event::Cut => Some(0x1B),     // X
                egui::Event::Paste(_) => Some(0x19), // V
                _ => None,
            } {
                // The shortcut arrives resolved and **without** the
                // `modifiers` field, so the Ctrl has to be put in front here.
                // Without that the macro came out with the C before the Ctrl --
                // the wrong order, seen on screen: the Ctrl release only came
                // at the end of the frame.
                if !self.down.contains(&0xE0) {
                    self.push(true, 0xE0, at);
                    // The release comes out on its own in the end-of-frame
                    // synchronization, when `ctrl` goes back to false.
                    self.modifiers.ctrl = true;
                }
                // Press and release at the same instant: `egui` hands the
                // shortcut over already resolved, without the touch duration.
                self.push(true, usage, at);
                self.push(false, usage, at);
                any = true;
                continue;
            }

            if let egui::Event::Key { key, physical_key, pressed, repeat, .. } = e {
                if *repeat {
                    continue;
                }
                let Some(usage) = usage_of(physical_key.unwrap_or(*key)) else { continue };
                // Auto-repeat the system sends without the `repeat` flag, and
                // the release of a key that was already up: neither of the two
                // is a new event.
                if *pressed == self.down.contains(&usage) {
                    continue;
                }
                self.push(*pressed, usage, at);
                any = true;
            }
        }

        // The end of the frame closes what no event carried -- releasing the
        // Shift on its own, for instance, generates no key event at all.
        any |= self.sync_modifiers(final_modifiers, at);
        any
    }

    /// Synchronizes the recorded modifiers with `m`, emitting press and
    /// release.
    ///
    /// A modifier is an ordinary key on the wire: `0xE0` goes down and up like
    /// any other, there is no modifier field (`docs/21` §2).
    ///
    /// `command` is deliberately left out. On Linux and Windows it is an alias
    /// of `ctrl`, so using it for Super generated a phantom `LWin` release
    /// every time Ctrl came up -- seen on screen. `mac_cmd` is the field that
    /// really means Super, and it only exists on macOS. The honest
    /// consequence: the Win key is **not recordable** here, because `egui`
    /// delivers it neither as an event nor as a field. Anyone who needs it in
    /// a macro records with the MR key on the keyboard itself.
    fn sync_modifiers(&mut self, m: egui::Modifiers, at: u64) -> bool {
        let mut any = false;
        for (was, is, usage) in [
            (self.modifiers.ctrl, m.ctrl, 0xE0u8),
            (self.modifiers.shift, m.shift, 0xE1),
            (self.modifiers.alt, m.alt, 0xE2),
            (self.modifiers.mac_cmd, m.mac_cmd, 0xE3),
        ] {
            if was != is {
                self.push(is, usage, at);
                any = true;
            }
        }
        self.modifiers = m;
        any
    }

    fn push(&mut self, press: bool, usage: u8, at: u64) {
        if self.raw.len() >= MAX_EVENTS {
            return;
        }
        if press {
            self.down.push(usage);
        } else {
            self.down.retain(|&u| u != usage);
        }
        self.raw.push((press, usage, at));
    }
}

/// The modifiers an event carries, when it carries any.
///
/// Only input events bring the field; `Copy`, `Cut` and `Paste` do not, and
/// for those the previous state still holds.
fn event_modifiers(e: &egui::Event) -> Option<egui::Modifiers> {
    match e {
        egui::Event::Key { modifiers, .. }
        | egui::Event::PointerButton { modifiers, .. }
        | egui::Event::MouseWheel { modifiers, .. } => Some(*modifiers),
        _ => None,
    }
}

/// HID usage of an `egui` key, when there is one.
///
/// `None` for what the keyboard cannot emit -- `egui::Key`'s browser and media
/// keys, which on the wire are a **key function** (`0xA5`-`0xAB`) and not a
/// HID usage, and do not fit inside a macro.
pub fn usage_of(key: egui::Key) -> Option<u8> {
    use egui::Key as K;
    Some(match key {
        K::A => 0x04, K::B => 0x05, K::C => 0x06, K::D => 0x07,
        K::E => 0x08, K::F => 0x09, K::G => 0x0A, K::H => 0x0B,
        K::I => 0x0C, K::J => 0x0D, K::K => 0x0E, K::L => 0x0F,
        K::M => 0x10, K::N => 0x11, K::O => 0x12, K::P => 0x13,
        K::Q => 0x14, K::R => 0x15, K::S => 0x16, K::T => 0x17,
        K::U => 0x18, K::V => 0x19, K::W => 0x1A, K::X => 0x1B,
        K::Y => 0x1C, K::Z => 0x1D,
        K::Num1 => 0x1E, K::Num2 => 0x1F, K::Num3 => 0x20, K::Num4 => 0x21,
        K::Num5 => 0x22, K::Num6 => 0x23, K::Num7 => 0x24, K::Num8 => 0x25,
        K::Num9 => 0x26, K::Num0 => 0x27,
        K::Enter => 0x28,
        K::Escape => 0x29,
        K::Backspace => 0x2A,
        K::Tab => 0x2B,
        K::Space => 0x2C,
        K::Minus => 0x2D,
        K::Equals | K::Plus => 0x2E,
        K::OpenBracket | K::OpenCurlyBracket => 0x2F,
        K::CloseBracket | K::CloseCurlyBracket => 0x30,
        K::Backslash | K::Pipe => 0x31,
        K::Semicolon | K::Colon => 0x33,
        K::Quote => 0x34,
        K::Backtick => 0x35,
        K::Comma => 0x36,
        K::Period => 0x37,
        K::Slash | K::Questionmark => 0x38,
        K::F1 => 0x3A, K::F2 => 0x3B, K::F3 => 0x3C, K::F4 => 0x3D,
        K::F5 => 0x3E, K::F6 => 0x3F, K::F7 => 0x40, K::F8 => 0x41,
        K::F9 => 0x42, K::F10 => 0x43, K::F11 => 0x44, K::F12 => 0x45,
        K::Insert => 0x49,
        K::Home => 0x4A,
        K::PageUp => 0x4B,
        K::Delete => 0x4C,
        K::End => 0x4D,
        K::PageDown => 0x4E,
        K::ArrowRight => 0x4F,
        K::ArrowLeft => 0x50,
        K::ArrowDown => 0x51,
        K::ArrowUp => 0x52,
        // F13-F24 are the free keys of `docs/28`: the keyboard emits them and
        // nothing on the system uses them out of the box.
        K::F13 => 0x68, K::F14 => 0x69, K::F15 => 0x6A, K::F16 => 0x6B,
        K::F17 => 0x6C, K::F18 => 0x6D, K::F19 => 0x6E, K::F20 => 0x6F,
        K::F21 => 0x70, K::F22 => 0x71, K::F23 => 0x72, K::F24 => 0x73,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// It is not just checking numbers: the map has to agree with the function
    /// table, which came from another source (`docs/10`). If the two disagree,
    /// recording `A` would put another key in the macro.
    #[test]
    fn the_map_agrees_with_the_function_table() {
        for (key, name) in [
            (egui::Key::A, "A"),
            (egui::Key::Z, "Z"),
            (egui::Key::Num0, "0"),
            (egui::Key::Enter, "Enter"),
            (egui::Key::Escape, "Esc"),
            (egui::Key::Space, "Space"),
            (egui::Key::Tab, "Tab"),
            (egui::Key::F1, "F1"),
            (egui::Key::F12, "F12"),
            (egui::Key::F13, "F13"),
            (egui::Key::ArrowUp, "Up"),
            (egui::Key::Delete, "Delete"),
        ] {
            let usage = usage_of(key).expect("mapped");
            assert_eq!(crate::assign::name(usage), name, "{key:?}");
        }
    }

    /// An `egui` media key is not a HID usage: on the wire it is a key
    /// function, range `0xA5`-`0xAB`, which does not go into a macro.
    #[test]
    fn media_keys_are_not_recordable() {
        assert!(usage_of(egui::Key::BrowserBack).is_none());
    }

    /// Auto-repeat arrives as a press with no release in between. Recording
    /// each one would fill the 84 events with the same key.
    #[test]
    fn autorepeat_does_not_become_events() {
        let mut r = Recorder::default();
        r.start(0.0);
        r.push(true, 0x04, 0);
        r.push(true, 0x04, 30); // the `collect` filter is what blocks it
        assert_eq!(r.len(), 2); // `push` is raw on purpose
        let mut r = Recorder::default();
        r.start(0.0);
        r.push(true, 0x04, 0);
        assert!(r.down.contains(&0x04));
        r.push(false, 0x04, 60);
        assert!(r.down.is_empty());
    }

    #[test]
    fn recording_stops_at_the_packet_limit() {
        let mut r = Recorder::default();
        r.start(0.0);
        for i in 0..200u64 {
            r.push(i % 2 == 0, 0x04, i * 10);
        }
        assert_eq!(r.len(), MAX_EVENTS);
        assert!(r.full());
    }
}
