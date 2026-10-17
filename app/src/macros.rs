//! Macros: the content of command `0x05` and the file they live in.
//!
//! The format is settled (`docs/09` §7b, validated in `docs/16` §1 and
//! `docs/21` §1 and §2): from byte 8 on comes a 2-byte header and then
//! 3-byte records.
//!
//!     [flag] [delay] [HID usage]
//!
//! - **flag** -- bit 7 set = press, clear = release. The **low nibble**
//!   carries the high bits of the delay.
//! - **delay** -- `((flag & 0x0F) << 8) | byte`, in units of **10 ms**,
//!   counted **before** the event. A single byte would saturate at 2.55 s;
//!   with the extension the ceiling goes to 4095 x 10 ms = ~41 s.
//! - **HID usage** -- the key. A modifier is an ordinary key: `Ctrl` goes in
//!   as `0xE0` and out as `0xE0`, there is no modifier field.
//!
//! ## Why the delay is stored in units of 10 ms, and not in ms
//!
//! Because that is the wire's unit. Storing it in ms would force rounding on
//! recording and on writing, and two roundings over the same number do not
//! commute -- a macro recorded, saved and reloaded could drift on its own. The
//! UI converts to seconds when displaying.
//!
//! ## What this module does NOT do
//!
//! It does not know **which** macro a key runs. The key gets the code `0xB4`
//! (`docs/16` §3) and the link to the macro index is `[I]`: the hypothesis is
//! the `0x05` header, never measured. That is why `Macro` keeps the header as
//! it came instead of pretending to understand it, and assigning `0xB4` stays
//! an explicit choice by someone who knows what they are doing.

use std::path::PathBuf;

use crate::proto;

/// File format version. Goes up when the content changes shape.
const FORMAT: u64 = 1;

/// Header bytes before the first record (`00 01` in the captures).
pub const HEADER: usize = 2;
/// Size of one record on the wire.
pub const RECORD: usize = 3;
/// How many events fit in a packet. 264 - 8 - 2 = 254, 254 / 3 = 84.
pub const MAX_EVENTS: usize = (proto::PKT - proto::HDR - HEADER) / RECORD;
/// Delay ceiling: 12 bits in units of 10 ms, ~41 s.
pub const MAX_DELAY: u16 = 0x0FFF;

/// One macro event. `delay` is in units of 10 ms and applies **before** the
/// event, as on the wire.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Event {
    pub press: bool,
    pub delay: u16,
    pub usage: u8,
}

impl Event {
    pub fn seconds(&self) -> f32 {
        self.delay as f32 / 100.0
    }

    /// Key name from the HID usage, for the screen. Falls back to hex when the
    /// code is not in the keyboard catalog -- a `0xB4` inside a macro is no key
    /// at all, and lying about the name would be worse than showing the number.
    pub fn key_name(&self) -> String {
        crate::assign::name(self.usage)
    }
}

/// A whole macro: name, header as it came off the wire, and the events.
#[derive(Clone, Debug, PartialEq)]
pub struct Macro {
    pub name: String,
    /// Bytes 8 and 9. `[0x00, 0x01]` in every capture; the exact meaning is
    /// `[I]` (`docs/09` §7b), so it is preserved instead of recreated.
    pub header: [u8; 2],
    pub events: Vec<Event>,
}

impl Macro {
    pub fn named(name: &str) -> Macro {
        Macro { name: name.to_string(), header: [0x00, 0x01], events: Vec::new() }
    }

    /// Total duration, in seconds -- the sum of the delays.
    pub fn duration(&self) -> f32 {
        self.events.iter().map(|e| e.delay as u32).sum::<u32>() as f32 / 100.0
    }

    /// Keys left stuck: a press with no release after it.
    ///
    /// This really matters. A macro that presses Ctrl and never releases it
    /// leaves Ctrl stuck **in the system**, not in the app, and the only way to
    /// notice is that everything stops working. The UI warns before recording.
    pub fn stuck_keys(&self) -> Vec<u8> {
        let mut down: Vec<u8> = Vec::new();
        for e in &self.events {
            if e.press {
                if !down.contains(&e.usage) {
                    down.push(e.usage);
                }
            } else {
                down.retain(|&u| u != e.usage);
            }
        }
        down
    }
}

/// Builds the `0x05` payload from byte 8 on.
///
/// Returns the 256 payload bytes, with the tail zeroed -- the zero is what
/// ends the list, as per the captures.
pub fn encode(m: &Macro) -> Result<Vec<u8>, String> {
    if m.events.len() > MAX_EVENTS {
        return Err(format!(
            "macro has {} events; {MAX_EVENTS} is what fits in one packet",
            m.events.len()
        ));
    }
    let mut out = vec![0u8; proto::PKT - proto::HDR];
    out[0] = m.header[0];
    out[1] = m.header[1];
    for (i, e) in m.events.iter().enumerate() {
        if e.delay > MAX_DELAY {
            return Err(format!(
                "event {i} has a delay of {} x 10 ms; the field holds {MAX_DELAY}",
                e.delay
            ));
        }
        if e.usage == 0 {
            // A record with usage 0 is what marks the end of the list, so an
            // event with usage 0 in the middle would truncate the macro on read.
            return Err(format!("event {i} has HID usage 0x00, which ends the list"));
        }
        let at = HEADER + i * RECORD;
        out[at] = (if e.press { 0x80 } else { 0x00 }) | ((e.delay >> 8) as u8 & 0x0F);
        out[at + 1] = (e.delay & 0xFF) as u8;
        out[at + 2] = e.usage;
    }
    Ok(out)
}

/// `true` when the bank has never been written.
///
/// `0x85` answers `0xFF` for a virgin bank -- measured on mode 3 of this
/// keyboard, which the official app never touched, while modes 1 and 2
/// returned the macros from the captures. It is the same convention `0x8A`
/// uses in the init (`docs/20`): `0xFF` means "there is nothing here", and it
/// is different from zeroed, which is an **empty** macro.
///
/// The test looks only at the **header**, and that was measured: the answer
/// comes back one byte short (263 of 264), so the last byte keeps the zero
/// from our own buffer and "all 256 are 0xFF" is never true. A written bank
/// has the header `00 01`; `FF FF` is what the firmware leaves where it never
/// wrote.
///
/// Without this, a virgin bank decodes as 84 events of `0xFF` with 40.95 s of
/// delay each -- a list full of garbage instead of an absent list.
pub fn absent(payload: &[u8]) -> bool {
    payload.len() >= HEADER && payload[0] == 0xFF && payload[1] == 0xFF
}

/// Reads the `0x05`/`0x85` payload from byte 8 on.
///
/// Stops at the first record with HID usage `0x00`: that is what the captures
/// show in the packet's tail, and `encode` depends on it.
pub fn decode(payload: &[u8]) -> Macro {
    let mut m = Macro::named("");
    if payload.len() >= HEADER {
        m.header = [payload[0], payload[1]];
    }
    let mut at = HEADER;
    while at + RECORD <= payload.len() && m.events.len() < MAX_EVENTS {
        let (flag, delay, usage) = (payload[at], payload[at + 1], payload[at + 2]);
        if usage == 0 {
            break;
        }
        m.events.push(Event {
            press: flag & 0x80 != 0,
            delay: ((flag as u16 & 0x0F) << 8) | delay as u16,
            usage,
        });
        at += RECORD;
    }
    m
}

/// Turns a raw recording -- `(press, usage, instant in ms since the start)` --
/// into the macro's events.
///
/// The first event always comes out with delay 0: the time between pressing
/// `Record` and the first key belongs to the user, not to the macro. The
/// following delays are the difference between instants, in units of 10 ms,
/// saturated at the field's ceiling.
pub fn from_recording(raw: &[(bool, u8, u64)]) -> Vec<Event> {
    let mut out = Vec::with_capacity(raw.len());
    let mut previous: Option<u64> = None;
    for &(press, usage, at) in raw {
        let delay = match previous {
            None => 0,
            Some(p) => (((at.saturating_sub(p)) + 5) / 10).min(MAX_DELAY as u64) as u16,
        };
        previous = Some(at);
        out.push(Event { press, delay, usage });
    }
    out
}

fn folder() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
    Some(base.join("km780").join("macros"))
}

pub fn path() -> String {
    folder().map(|p| p.display().to_string()).unwrap_or_default()
}

/// Safe file name -- the same rule as the profiles, for the same reason.
fn sanitize(name: &str) -> Option<String> {
    let s: String = name
        .trim()
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '-' })
        .collect();
    let s = s.trim_matches('-').to_string();
    if s.is_empty() { None } else { Some(s) }
}

pub fn list() -> Vec<String> {
    let Some(p) = folder() else { return Vec::new() };
    let Ok(rd) = std::fs::read_dir(&p) else { return Vec::new() };
    let mut v: Vec<String> = rd
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let n = e.file_name().to_string_lossy().to_string();
            n.strip_suffix(".json").map(str::to_string)
        })
        .collect();
    v.sort();
    v
}

pub fn save(m: &Macro) -> Result<String, String> {
    let name = sanitize(&m.name).ok_or("empty macro name")?;
    let dir = folder().ok_or("missing HOME and XDG_CONFIG_HOME")?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;

    let events: Vec<serde_json::Value> = m
        .events
        .iter()
        .map(|e| {
            let mut o = serde_json::Map::new();
            o.insert("press".into(), e.press.into());
            o.insert("delay".into(), e.delay.into());
            o.insert("usage".into(), e.usage.into());
            serde_json::Value::Object(o)
        })
        .collect();
    let mut o = serde_json::Map::new();
    o.insert("format".into(), FORMAT.into());
    o.insert("name".into(), name.clone().into());
    o.insert("header".into(), serde_json::json!([m.header[0], m.header[1]]));
    // Recorded in the file only for whoever reads the JSON by hand: the field
    // does not come back on read, the delay is the source.
    o.insert("unit_ms".into(), 10.into());
    o.insert("events".into(), serde_json::Value::Array(events));

    let file_path = dir.join(format!("{name}.json"));
    let text = serde_json::to_string_pretty(&serde_json::Value::Object(o))
        .map_err(|e| e.to_string())?;
    std::fs::write(&file_path, text).map_err(|e| format!("{}: {e}", file_path.display()))?;
    Ok(file_path.display().to_string())
}

pub fn load(name: &str) -> Result<Macro, String> {
    let name = sanitize(name).ok_or("empty macro name")?;
    let file_path = folder()
        .ok_or("missing HOME and XDG_CONFIG_HOME")?
        .join(format!("{name}.json"));
    let text = std::fs::read_to_string(&file_path)
        .map_err(|e| format!("{}: {e}", file_path.display()))?;
    let v: serde_json::Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;

    match v.get("format").and_then(|f| f.as_u64()) {
        Some(f) if f <= FORMAT => {}
        Some(f) => return Err(format!("macro is format {f}; this version reads up to {FORMAT}")),
        None => return Err("file has no 'format' field \u{2014} not a km780 macro".into()),
    }

    let mut m = Macro::named(&name);
    if let Some(h) = v.get("header").and_then(|h| h.as_array()) {
        if h.len() == 2 {
            m.header = [
                h[0].as_u64().unwrap_or(0) as u8,
                h[1].as_u64().unwrap_or(1) as u8,
            ];
        }
    }
    let arr = v
        .get("events")
        .and_then(|e| e.as_array())
        .ok_or("macro has no 'events' list")?;
    for (i, e) in arr.iter().enumerate() {
        let num = |k: &str| e.get(k).and_then(|x| x.as_u64());
        let usage = num("usage")
            .filter(|&u| (1..=255).contains(&u))
            .ok_or_else(|| format!("event {i} has no valid 'usage'"))?;
        let delay = num("delay")
            .filter(|&d| d <= MAX_DELAY as u64)
            .ok_or_else(|| format!("event {i} has no valid 'delay'"))?;
        let press = e
            .get("press")
            .and_then(|p| p.as_bool())
            .ok_or_else(|| format!("event {i} has no valid 'press'"))?;
        m.events.push(Event { press, delay: delay as u16, usage: usage as u8 });
    }
    if m.events.len() > MAX_EVENTS {
        return Err(format!("macro has {} events; the limit is {MAX_EVENTS}", m.events.len()));
    }
    Ok(m)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The bytes from byte 10 of the capture, as `docs/16` §1 lists them.
    /// The macro on the official app's screen was:
    ///     A | 0.07 s | A | 1.68 s | B | 0.07 s | B | 2.07 s | C | 0.09 s | C
    const ABC: &[u8] = &[
        0x80, 0x00, 0x04, 0x00, 0x07, 0x04, 0x80, 0xA8, 0x05,
        0x00, 0x07, 0x05, 0x80, 0xCF, 0x06, 0x00, 0x09, 0x06,
    ];

    fn payload(records: &[u8]) -> Vec<u8> {
        let mut v = vec![0x00, 0x01];
        v.extend_from_slice(records);
        v.resize(proto::PKT - proto::HDR, 0);
        v
    }

    #[test]
    fn decodes_the_abc_capture() {
        let m = decode(&payload(ABC));
        assert_eq!(m.header, [0x00, 0x01]);
        let expected = [
            (true, 0, 0x04),   //  press A, +0 ms
            (false, 7, 0x04),  //  release A, 0.07 s
            (true, 168, 0x05), //  press B, 1.68 s
            (false, 7, 0x05),
            (true, 207, 0x06), //  press C, 2.07 s
            (false, 9, 0x06),
        ];
        assert_eq!(m.events.len(), expected.len());
        for (e, (press, delay, usage)) in m.events.iter().zip(expected) {
            assert_eq!((e.press, e.delay, e.usage), (press, delay, usage));
        }
    }

    /// `docs/21` §1: an 8.65 s pause via the flag's low nibble.
    #[test]
    fn decodes_the_extended_delay() {
        let m = decode(&payload(&[
            0x80, 0x00, 0x04, 0x00, 0x06, 0x04, 0x83, 0x61, 0x05, 0x00, 0x07, 0x05,
        ]));
        assert_eq!(m.events[2].delay, 865);
        assert!((m.events[2].seconds() - 8.65).abs() < 0.001);
        assert!(m.events[2].press);
    }

    /// `docs/21` §2: Ctrl+C. The modifier is an ordinary key, `0xE0`.
    #[test]
    fn decodes_ctrl_c_with_no_modifier_field() {
        let m = decode(&payload(&[
            0x80, 0x00, 0xE0, 0x80, 0xCF, 0x06, 0x00, 0x06, 0x06, 0x00, 0x3B, 0xE0,
        ]));
        assert_eq!(m.events.iter().map(|e| e.usage).collect::<Vec<_>>(), [0xE0, 0x06, 0x06, 0xE0]);
        assert_eq!(m.events[3].delay, 59);
        assert!(m.stuck_keys().is_empty());
    }

    /// Round trip through the capture's bytes: what `encode` builds has to be
    /// identical to what the official app sent on the wire.
    #[test]
    fn encode_rebuilds_the_captured_bytes() {
        let bytes = payload(ABC);
        let m = decode(&bytes);
        assert_eq!(encode(&m).unwrap(), bytes);
    }

    #[test]
    fn a_press_with_no_release_is_reported() {
        let mut m = Macro::named("stuck");
        m.events = vec![
            Event { press: true, delay: 0, usage: 0xE0 },
            Event { press: true, delay: 10, usage: 0x06 },
            Event { press: false, delay: 5, usage: 0x06 },
        ];
        assert_eq!(m.stuck_keys(), vec![0xE0]);
    }

    #[test]
    fn the_packet_holds_84_events() {
        assert_eq!(MAX_EVENTS, 84);
        let mut m = Macro::named("full");
        m.events = vec![Event { press: true, delay: 1, usage: 0x04 }; MAX_EVENTS + 1];
        assert!(encode(&m).is_err());
        m.events.pop();
        assert!(encode(&m).is_ok());
    }

    /// A usage 0 in the middle would end the list on re-read, so `encode`
    /// refuses instead of writing a macro that comes back truncated.
    #[test]
    fn usage_zero_is_refused() {
        let mut m = Macro::named("zero");
        m.events = vec![Event { press: true, delay: 0, usage: 0x00 }];
        assert!(encode(&m).is_err());
    }

    #[test]
    fn recording_starts_with_no_delay_and_rounds_to_10ms() {
        let raw = [(true, 0x04, 1234), (false, 0x04, 1304), (true, 0x05, 2984)];
        let ev = from_recording(&raw);
        assert_eq!(ev[0].delay, 0);
        assert_eq!(ev[1].delay, 7);   // 70 ms
        assert_eq!(ev[2].delay, 168); // 1680 ms
    }

    #[test]
    fn a_long_pause_saturates_instead_of_wrapping() {
        let ev = from_recording(&[(true, 0x04, 0), (true, 0x05, 120_000)]);
        assert_eq!(ev[1].delay, MAX_DELAY);
    }

    /// Measured on mode 3, which the official app never wrote to.
    #[test]
    fn an_untouched_bank_is_absent_not_full() {
        let mut virgin = vec![0xFFu8; proto::PKT - proto::HDR];
        assert!(absent(&virgin));
        // How it really arrives: the answer comes back one byte short, so the
        // last one stays zeroed.
        *virgin.last_mut().unwrap() = 0x00;
        assert!(absent(&virgin));
        // Without `absent`, this is what the raw decoding would see:
        assert_eq!(decode(&virgin).events.len(), MAX_EVENTS);
        assert!(!absent(&payload(ABC)));
        // Zeroed is an empty macro, not an absent bank.
        assert!(!absent(&vec![0u8; proto::PKT - proto::HDR]));
    }

    #[test]
    fn name_cannot_escape_the_folder() {
        assert_eq!(sanitize("../../etc/passwd").as_deref(), Some("etc-passwd"));
        assert_eq!(sanitize("  ").as_deref(), None);
    }
}
