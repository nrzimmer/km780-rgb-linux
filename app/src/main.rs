//! Lighting control for the G.SKILL Ripjaws KM780 RGB.
//!
//! v1: reads the keyboard and writes static color, in batch, with an explicit
//! `Apply`. The protocol is in `docs/09-protocol.md`; the design decisions, in
//! `docs/29-app-rust.md`.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod effects;
mod layout;
mod proto;
mod slots;

/// `km780 --probe`: opens the keyboard and reads, with no GUI and writing
/// nothing. Only the confirmed read commands (0x81, 0x82, 0x89).
fn probe() {
    match proto::Keyboard::open_device() {
        Err(e) => {
            eprintln!("could not open: {e}");
            std::process::exit(1);
        }
        Ok(kb) => {
            println!("open: {}", kb.description);
            match kb.active_mode() {
                Ok(m) => {
                    println!("active mode: {m}");
                    match kb.read_mode(m) {
                        Ok(d) => println!(
                            "effect={} brightness={}",
                            d[proto::OFF_EFFECT], d[proto::OFF_BRIGHTNESS]
                        ),
                        Err(e) => println!("0x82 failed: {e}"),
                    }
                    match kb.read_colors(m) {
                        Ok(c) => {
                            let nonzero = (0..slots::NSLOTS)
                                .filter(|&i| c[0][i] != 0 || c[1][i] != 0 || c[2][i] != 0)
                                .count();
                            println!("0x89 ok: {nonzero} slots lit");
                            for name in ["F1", "F2", "F3", "1", "2", "3", "4"] {
                                if let Some(i) = slots::slot_of(name) {
                                    println!("  {name:4} slot {i:3}  #{:02X}{:02X}{:02X}",
                                             c[0][i], c[1][i], c[2][i]);
                                }
                            }
                        }
                        Err(e) => println!("0x89 failed: {e}"),
                    }
                }
                Err(e) => println!("0x81 failed: {e}"),
            }
        }
    }
}

/// `km780 --paint F1,F2,F3 ff0000`: reads, changes only the named slots,
/// writes, and **re-reads with 0x89 to check byte for byte**.
///
/// It exists to validate the write path without the GUI. Checking by re-read
/// is not excessive zeal: this unit has LEDs with the R component stuck
/// (`docs/27` §10), so the eye is no proof.
fn paint_cmd(keys: &str, color: &str) {
    let c = u32::from_str_radix(color.trim_start_matches('#'), 16)
        .expect("color em RRGGBB");
    let target = [(c >> 16) as u8, (c >> 8) as u8, c as u8];
    let indices: Vec<usize> = keys
        .split(',')
        .map(|n| slots::slot_of(n.trim()).unwrap_or_else(|| panic!("unknown key: {n}")))
        .collect();

    let kb = proto::Keyboard::open_device().expect("could not open");
    let mode = kb.active_mode().expect("0x81");
    let mut channels = kb.read_colors(mode).expect("0x89");

    println!("mode {mode}; previous color of the slots touched:");
    for &i in &indices {
        println!("  slot {i:3}  #{:02X}{:02X}{:02X}", channels[0][i], channels[1][i], channels[2][i]);
    }
    for &i in &indices {
        channels[0][i] = target[0];
        channels[1][i] = target[1];
        channels[2][i] = target[2];
    }
    kb.write_colors(mode, &channels).expect("0x09");

    let after = kb.read_colors(mode).expect("0x89 back");
    let mut diffs = 0;
    for i in 0..slots::NSLOTS {
        for ch in 0..3 {
            if after[ch][i] != channels[ch][i] {
                diffs += 1;
                println!("  DIVERGES slot {i} channel {ch}: {} != {}", after[ch][i], channels[ch][i]);
            }
        }
    }
    if diffs == 0 {
        println!("the re-read matches what was written, across the 144 slots of the 3 channels");
    }
}

/// `km780 --brightness 2` / `km780 --effect 4`: read-change-write of one byte
/// of the mode payload, and **a diff of the whole payload afterwards**.
///
/// The diff is the point. Rule 3 of `docs/29` requires the unknown fields to
/// travel intact through the round-trip; this proves they do, instead of
/// trusting that they do. Measured on mode 2: of the 264 bytes, exactly what
/// was asked for changes.
fn set_mode_byte(offset: usize, value: &str) {
    let v: u8 = value.parse().expect("numeric value");
    if offset == proto::OFF_BRIGHTNESS {
        assert!(v <= 4, "brightness is 0..4 (docs/09)");
    }

    let kb = proto::Keyboard::open_device().expect("could not open");
    let mode = kb.active_mode().expect("0x81");
    let before = kb.read_mode(mode).expect("0x82");
    println!(
        "mode {mode}: brightness {} effect {} -> byte {offset} = {v}",
        before[proto::OFF_BRIGHTNESS],
        before[proto::OFF_EFFECT]
    );

    let mut new_payload = before.clone();
    new_payload[offset] = v;
    kb.write_mode(mode, &new_payload).expect("0x02");

    let after = kb.read_mode(mode).expect("0x82 back");
    let diffs: Vec<usize> = (0..proto::PKT)
        .filter(|&i| after[i] != before[i])
        .collect();
    println!("bytes that changed in the 264-byte payload: {diffs:?}");
    for i in diffs {
        println!("  byte {i}: {:#04x} -> {:#04x}", before[i], after[i]);
    }
}

fn main() -> eframe::Result<()> {
    let argv0: Vec<String> = std::env::args().collect();
    if let Some(p) = argv0.iter().position(|a| a == "--brightness") {
        set_mode_byte(proto::OFF_BRIGHTNESS, &argv0[p + 1]);
        return Ok(());
    }
    if let Some(p) = argv0.iter().position(|a| a == "--effect") {
        set_mode_byte(proto::OFF_EFFECT, &argv0[p + 1]);
        return Ok(());
    }

    let argv: Vec<String> = std::env::args().collect();
    if let Some(p) = argv.iter().position(|a| a == "--paint") {
        paint_cmd(&argv[p + 1], &argv[p + 2]);
        return Ok(());
    }

    if std::env::args().any(|a| a == "--segura") {
        // Diagnostics: open and hold, to measure what the host loses
        // while interface 1 is claimed.
        let _kb = proto::Keyboard::open_device().expect("could not open");
        println!("holding for 8s");
        std::thread::sleep(std::time::Duration::from_secs(8));
        return Ok(());
    }
    if std::env::args().any(|a| a == "--probe") {
        probe();
        return Ok(());
    }

    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1180.0, 560.0])
            .with_min_inner_size([760.0, 420.0]),
        ..Default::default()
    };
    eframe::run_native(
        "KM780 RGB",
        options,
        Box::new(|cc| Ok(Box::new(app::App::new(cc)))),
    )
}
