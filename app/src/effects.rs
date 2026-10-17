//! Effect catalog: byte 10 of the mode payload.
//!
//! The canonical table is the one in `docs/09` §7.1, the union of what the
//! captures saw with what decompiling the builder confirmed (`docs/27` §7).
//!
//! `0x05` is not here: it has no branch in the DLL and never showed up in a
//! capture -- it probably does not exist. `0x06` is here, and it is the odd
//! one: it also has no branch in the DLL and the official UI does not offer
//! it, yet the firmware runs it. Confirmed on hardware in `docs/27` §10.

/// `(byte 10, name, note)`.
pub static CATALOG: &[(u8, &str, &str)] = &[
    (0x00, "none (static color)", "slot colors show exactly as painted"),
    (0x01, "Wave", ""),
    (0x02, "Ripple", "color comes from the gradient already stored (0x0B)"),
    (0x03, "Sonic", "color comes from the gradient already stored (0x0B)"),
    (0x04, "Breathing", ""),
    (
        0x06,
        "key lights on press",
        "outside the official UI and missing a branch in the DLL; firmware only (docs/27 §10)",
    ),
    (0x07, "Changeling", "color comes from the gradient already stored (0x0E/0x15)"),
    (0x08, "Checkpoint", "uses Transition Point instead of duration/direction"),
    (0x09, "Cycle", ""),
    (0x0A, "Rain", "color comes from the gradient already stored (0x0F/0x12)"),
];

pub fn name(b: u8) -> String {
    match CATALOG.iter().find(|(v, _, _)| *v == b) {
        Some((_, n, _)) => (*n).to_string(),
        // 0x05 and any other value the keyboard returns: showing the raw
        // number beats inventing a name.
        None => format!("unknown (0x{b:02X})"),
    }
}

pub fn note(b: u8) -> &'static str {
    CATALOG
        .iter()
        .find(|(v, _, _)| *v == b)
        .map(|(_, _, o)| *o)
        .unwrap_or("")
}
