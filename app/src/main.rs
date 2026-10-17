//! Lighting control for the G.SKILL Ripjaws KM780 RGB.
//!
//! v1: reads the keyboard and writes static color, in batch, with an explicit
//! `Apply`. The protocol is in `docs/09-protocol.md`; the design decisions, in
//! `docs/29-app-rust.md`.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod assign;
mod effects;
mod layout;
mod macros;
mod profiles;
mod proto;
mod recorder;
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
            println!("opened: {}", kb.description);
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
                            let lit = (0..slots::NSLOTS)
                                .filter(|&i| c[0][i] != 0 || c[1][i] != 0 || c[2][i] != 0)
                                .count();
                            println!("0x89 ok: {lit} slots lit");
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

    println!("mode {mode}; previous color of the touched slots:");
    for &i in &indices {
        // A warning, not a refusal: painting a slot with no key is legitimate
        // -- several of them are lit on this keyboard -- but whoever asked by
        // number deserves to know they will see nothing on a US/ABNT layout.
        let note = if slots::NO_PHYSICAL_KEY.contains(&i) {
            "   (no physical key on the US/ABNT layout)"
        } else {
            ""
        };
        println!(
            "  slot {i:3}  #{:02X}{:02X}{:02X}{note}",
            channels[0][i], channels[1][i], channels[2][i]
        );
    }
    for &i in &indices {
        channels[0][i] = target[0];
        channels[1][i] = target[1];
        channels[2][i] = target[2];
    }
    kb.write_colors(mode, &channels).expect("0x09");

    let after = kb.read_colors(mode).expect("0x89 de back");
    let mut diffs = 0;
    for i in 0..slots::NSLOTS {
        for ch in 0..3 {
            if after[ch][i] != channels[ch][i] {
                diffs += 1;
                println!("  MISMATCH slot {i} channel {ch}: {} != {}", after[ch][i], channels[ch][i]);
            }
        }
    }
    if diffs == 0 {
        println!("read-back matches what was written, across all 144 slots of the 3 channels");
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

    let mut updated = before.clone();
    updated[offset] = v;
    kb.write_mode(mode, &updated).expect("0x02");

    let after = kb.read_mode(mode).expect("0x82 de back");
    let diffs: Vec<usize> = (0..proto::PKT)
        .filter(|&i| after[i] != before[i])
        .collect();
    println!("bytes changed in the 264-byte payload: {diffs:?}");
    for i in diffs {
        println!("  byte {i}: {:#04x} -> {:#04x}", before[i], after[i]);
    }
}

/// `km780 --function Timer` reads; `km780 --function Timer=f14` writes **to
/// all three modes**, saving the previous value.
///
/// Same semantics as `tools/km780assign.py`, including the record in
/// `~/.config/km780/assignments.json` -- the two have to see what the other
/// stored, otherwise one's `restore` does not undo the other's.
fn function(arg: &str) {
    let (key_name, updated) = match arg.split_once('=') {
        Some((t, c)) => {
            let c = c.trim().to_lowercase();
            let v = if let Some(n) = c.strip_prefix('f') {
                let n: u8 = n.parse().expect("f13..f24");
                assert!((13..=24).contains(&n), "only F13..F24 by name");
                0x68 + (n - 13)
            } else {
                u8::from_str_radix(&c, 16).expect("hex, e.g. 68")
            };
            (t.trim().to_string(), Some(v))
        }
        None => (arg.trim().to_string(), None),
    };
    let slot = slots::slot_of(&key_name).expect("unknown key");
    let off = assign::OFF_FUNCTIONS + slot;

    let kb = proto::Keyboard::open_device().expect("could not open");
    let record = assign::Record::load();

    // Read: all three modes, because the table is per mode.
    for mode in 1..=3u8 {
        let d = kb.read_mode(mode).expect("0x82");
        let before = d[off];
        match updated {
            None => println!(
                "mode {mode}  {key_name}  byte {off} = {} ({}){}",
                assign::name(before),
                assign::range_name(before),
                match record.previous(mode, slot) {
                    Some(v) => format!("   recorded previous: {}", assign::name(v)),
                    None => String::new(),
                }
            ),
            Some(_) => {}
        }
    }
    let Some(v) = updated else { return };

    // Writing through the SAME path as the GUI: the active mode via write_mode,
    // the other two via assign::propagate. That way the on-the-wire test done
    // here holds for the window's `Apply` too.
    let mode = kb.active_mode().expect("0x81");
    let before_payload = kb.read_mode(mode).expect("0x82");
    let mut after = before_payload.clone();
    after[off] = v;
    kb.write_mode(mode, &after).expect("0x02");
    let changed = assign::propagate(&kb, mode, &before_payload, &after).expect("propagate");

    for m in 1..=3u8 {
        let back = kb.read_mode(m).expect("0x82 de back");
        println!(
            "mode {m}  {key_name}  byte {off} = {}{}",
            assign::name(back[off]),
            if back[off] == v { "" } else { "   DID NOT STICK" }
        );
    }
    println!(
        "{} slot(s) propagated to all 3 modes; previous values in {}",
        changed.len(),
        assign::Record::path()
    );
}

/// `km780 --macro-dump`: reads the macro banks of the three modes with `0x85`
/// and shows what it decoded. **Read only**, nothing is written.
///
/// It serves two purposes. The first is seeing whether there is a macro
/// recorded with the MR key -- that recording happens inside the keyboard,
/// with no host, and has never been read by this project. The second is
/// finding out what `0x85` returns: the command shows up in the init and in
/// the decompilation, but the capture never showed a macro being **read**,
/// only written. Whatever comes out of here is a new measurement.
fn macro_dump() {
    let kb = proto::Keyboard::open_device().expect("could not open");
    println!("opened: {}\n", kb.description);
    for mode in 1..=3u8 {
        for bank in proto::MACRO_BANKS {
            match kb.read_macro(mode, 0, bank) {
                Err(e) => println!("mode {mode} bank {bank}: {e}"),
                Ok(payload) if macros::absent(&payload) => {
                    println!("mode {mode} bank {bank}: absent (0xFF) \u{2014} never written");
                }
                Ok(payload) => {
                    let m = macros::decode(&payload);
                    let used = payload.iter().filter(|&&b| b != 0).count();
                    println!(
                        "mode {mode} bank {bank}: header {:02X} {:02X}, \
                         {} event(s), {} non-zero byte(s)",
                        m.header[0], m.header[1], m.events.len(), used
                    );
                    for (i, e) in m.events.iter().enumerate() {
                        println!(
                            "   {i:2}  {}  {:>7.2}s  {}",
                            if e.press { "press  " } else { "release" },
                            e.seconds(),
                            e.key_name()
                        );
                    }
                    // The first raw bytes too, because the header is `[I]` and
                    // a field I do not understand should not appear only
                    // decoded.
                    let head: Vec<String> =
                        payload[..16].iter().map(|b| format!("{b:02x}")).collect();
                    println!("       raw: {}", head.join(" "));
                }
            }
        }
    }
}

/// `km780 --macro-import`: saves to file every macro the keyboard has.
///
/// The name comes from the address (`kb-m2-b1`), because the keyboard does
/// **not** store the name alongside: the name lives in database `0x0A` page
/// `0x09`, by another path (`docs/16` §2). Inventing a pretty name here would
/// hide that.
fn macro_import() {
    let kb = proto::Keyboard::open_device().expect("could not open");
    let mut saved = 0;
    for mode in 1..=3u8 {
        for bank in proto::MACRO_BANKS {
            let Ok(payload) = kb.read_macro(mode, 0, bank) else { continue };
            if macros::absent(&payload) {
                continue;
            }
            let mut m = macros::decode(&payload);
            if m.events.is_empty() {
                continue;
            }
            m.name = format!("kb-m{mode}-b{bank}");
            match macros::save(&m) {
                Ok(path) => {
                    println!("{} event(s) -> {path}", m.events.len());
                    saved += 1;
                }
                Err(e) => println!("mode {mode} bank {bank}: {e}"),
            }
        }
    }
    println!("{saved} macro(s) imported from the keyboard");
}

/// `km780 --macro-erase MODE`: returns a mode's macro banks to the
/// virgin default.
fn macro_erase(mode: u8) {
    assert!((1..=3).contains(&mode), "mode has to be 1, 2 or 3");
    let kb = proto::Keyboard::open_device().expect("could not open");
    kb.erase_macro(mode, 0).expect("0x05");
    std::thread::sleep(std::time::Duration::from_millis(500));
    for bank in proto::MACRO_BANKS {
        let state = match kb.read_macro(mode, 0, bank) {
            Err(e) => e,
            Ok(p) if macros::absent(&p) => "absent".into(),
            Ok(p) => format!("{} event(s) still there", macros::decode(&p).events.len()),
        };
        println!("mode {mode} bank {bank}: {state}");
    }
}

/// ASCII text at the start of a payload, if there is any.
///
/// The database stores names in pure ASCII, zero-terminated. Any byte outside
/// the printable range ends the read: it is a sign that what is there is not a
/// name but a binary parameter -- subpages `02`/`05`/`08` are like that.
fn ascii_prefix(payload: &[u8]) -> Option<String> {
    let text: String = payload
        .iter()
        .take_while(|&&b| b.is_ascii_graphic() || b == b' ')
        .map(|&b| b as char)
        .collect();
    if text.trim().is_empty() { None } else { Some(text) }
}

/// `km780 --base-read PAGE SUB`: reads **one** subpage, in hex and ASCII.
///
/// It exists to separate an address's content from the effect of read order: a
/// lone read, in a lone process, has no earlier read to inherit an answer from.
fn base_read(page: u8, sub: u8) {
    let kb = proto::Keyboard::open_device().expect("could not open");
    let d = kb.read_base(page, sub).expect("0x8a");
    let head: Vec<String> = d[..24].iter().map(|b| format!("{b:02x}")).collect();
    println!("page 0x{page:02x} sub 0x{sub:02x}");
    println!("  hex   {}", head.join(" "));
    println!("  ascii {:?}", ascii_prefix(&d));
}

/// `km780 --base-clear`: returns the name area of page `0x09` to the
/// virgin default.
fn base_clear() {
    let kb = proto::Keyboard::open_device().expect("could not open");
    let virgin = vec![0xFFu8; proto::PKT - proto::HDR];
    for sub in 0..proto::Keyboard::SUBPAGES {
        kb.write_base(proto::PAGE_MACRO_NAMES, sub, &virgin).expect("0x0a");
    }
    println!("page 0x09 cleared");
}

/// `km780 --base-dump`: reads the whole `0x0A` database with `0x8A` and shows
/// whatever is in each subpage. **Read only.**
///
/// The pages are the ones in `docs/09` §7c: `0x00` profile and mode names,
/// `0x09` macro and text names, and one per mode (`0x1b`/`0x1c`/`0x1d`).
fn base_dump() {
    let kb = proto::Keyboard::open_device().expect("could not open");
    println!("opened: {}\n", kb.description);

    let pages: Vec<(u8, u8, &str)> = vec![
        (proto::PAGE_NAMES, 5, "names: profile and modes"),
        (proto::PAGE_MACRO_NAMES, proto::Keyboard::SUBPAGES, "macro and text names"),
        (proto::PAGE_MODE[0], 11, "mode 1 base"),
        (proto::PAGE_MODE[1], 11, "mode 2 base"),
        (proto::PAGE_MODE[2], 11, "mode 3 base"),
    ];
    for (page, subs, what) in pages {
        println!("page 0x{page:02x} \u{2014} {what}");
        for sub in 0..subs {
            match kb.read_base(page, sub) {
                Err(e) => println!("  sub 0x{sub:02x}: {e}"),
                Ok(payload) => {
                    // The first two subpages of the names page do not keep
                    // what is written, and `0x00` still returns `0x02`'s
                    // content. Saying so here avoids reading the mirror as if
                    // it were one more name.
                    let note = if page == proto::PAGE_MACRO_NAMES && sub < proto::Keyboard::FIRST_NAME_SUB {
                        if sub == 0 { "   (mirror of sub 0x02; writes here are discarded)" }
                        else { "   (writes here are discarded)" }
                    } else {
                        ""
                    };
                    let used = payload.iter().filter(|&&b| b != 0).count();
                    if used == 0 {
                        println!("  sub 0x{sub:02x}: zeroed{note}");
                        continue;
                    }
                    if payload.iter().all(|&b| b == 0xFF) {
                        println!("  sub 0x{sub:02x}: 0xFF \u{2014} never written{note}");
                        continue;
                    }
                    match ascii_prefix(&payload) {
                        Some(text) => println!("  sub 0x{sub:02x}: {text:?}{note}"),
                        None => {
                            let head: Vec<String> =
                                payload[..16].iter().map(|b| format!("{b:02x}")).collect();
                            println!(
                                "  sub 0x{sub:02x}: {used} byte(s), {}",
                                head.join(" ")
                            );
                        }
                    }
                }
            }
        }
        println!();
    }
}

/// `km780 --macro-name NAME`: writes the name of macro index 0 into database
/// `0x0A` page `0x09`.
fn macro_name(name: &str, index: u8) {
    let kb = proto::Keyboard::open_device().expect("could not open");
    kb.write_macro_name(index, name).expect("0x0a");
    println!("macro {index} named {name:?} in the 0x0A page 0x09 base");
}

/// `km780 --macro-list`: the macros saved in files, without touching the keyboard.
fn macro_list() {
    let names = macros::list();
    if names.is_empty() {
        println!("no macro saved in {}", macros::path());
        return;
    }
    for name in names {
        match macros::load(&name) {
            Err(e) => println!("{name}: {e}"),
            Ok(m) => {
                let stuck = m.stuck_keys();
                println!(
                    "{name}: {} event(s), {:.2}s{}",
                    m.events.len(),
                    m.duration(),
                    if stuck.is_empty() {
                        String::new()
                    } else {
                        format!(
                            "   WARNING: key(s) never released: {}",
                            stuck.iter().map(|&u| assign::name(u)).collect::<Vec<_>>().join(", ")
                        )
                    }
                );
                for (i, e) in m.events.iter().enumerate() {
                    println!(
                        "   {i:2}  {}  {:>7.2}s  {}",
                        if e.press { "press  " } else { "release" },
                        e.seconds(),
                        e.key_name()
                    );
                }
            }
        }
    }
}

/// `km780 --macro-write NAME`: writes the saved macro into bank 1 of the
/// active mode and **re-reads with 0x85 to check**.
///
/// Checking by re-read is the same zeal as `--paint`: without it, "it worked"
/// only means the write did not error.
///
/// This does **not** assign the macro to any key. The code `0xB4` is the link,
/// and which macro it runs is `[I]` (`docs/16` §3) -- assigning it here would
/// be pretending the link is measured.
fn macro_write(name: &str, target: Option<u8>) {
    let m = macros::load(name).expect("could not load the macro");
    let payload = macros::encode(&m).expect("could not encode");
    let stuck = m.stuck_keys();
    if !stuck.is_empty() {
        println!(
            "WARNING: this macro never releases {} -- it will leave the key \
             held down in the system",
            stuck.iter().map(|&u| assign::name(u)).collect::<Vec<_>>().join(", ")
        );
    }

    let kb = proto::Keyboard::open_device().expect("could not open");
    let mode = match target {
        Some(m) => {
            assert!((1..=3).contains(&m), "mode has to be 1, 2 or 3");
            m
        }
        None => kb.active_mode().expect("0x81"),
    };
    println!("mode {mode}: writing {} event(s) to bank 1", m.events.len());
    // The official app's whole sequence: frame opened, the three banks, frame
    // closed. Banks 2 and 3 go out zeroed, as in the capture.
    kb.write_macro(mode, 0, &m.name, &payload).expect("0x05");

    // An immediate re-read returns the old content -- measured. Waiting is part
    // of the check, not ceremony.
    std::thread::sleep(std::time::Duration::from_millis(500));
    match kb.read_macro(mode, 0, 1) {
        Err(e) => println!("could not read back: {e}"),
        Ok(back) => {
            let d = macros::decode(&back);
            println!(
                "read back: {} event(s){}",
                d.events.len(),
                if d.events == m.events { "  (identical)" } else { "  DIFFERENT" }
            );
        }
    }
}

fn main() -> eframe::Result<()> {
    let argv: Vec<String> = std::env::args().collect();
    if let Some(p) = argv.iter().position(|a| a == "--brightness") {
        set_mode_byte(proto::OFF_BRIGHTNESS, &argv[p + 1]);
        return Ok(());
    }
    if let Some(p) = argv.iter().position(|a| a == "--function") {
        function(&argv[p + 1]);
        return Ok(());
    }
    if let Some(p) = argv.iter().position(|a| a == "--effect") {
        set_mode_byte(proto::OFF_EFFECT, &argv[p + 1]);
        return Ok(());
    }

    let argv: Vec<String> = std::env::args().collect();
    if let Some(p) = argv.iter().position(|a| a == "--paint") {
        paint_cmd(&argv[p + 1], &argv[p + 2]);
        return Ok(());
    }

    if std::env::args().any(|a| a == "--hold") {
        // Diagnostic: opens and holds on, to measure what the host loses while
        // interface 1 is claimed.
        let _kb = proto::Keyboard::open_device().expect("could not open");
        println!("holding for 8s");
        std::thread::sleep(std::time::Duration::from_secs(8));
        return Ok(());
    }
    if let Some(p) = argv.iter().position(|a| a == "--macro-write") {
        let target = argv.get(p + 2).and_then(|a| a.parse::<u8>().ok());
        macro_write(&argv[p + 1], target);
        return Ok(());
    }
    if let Some(p) = argv.iter().position(|a| a == "--macro-erase") {
        macro_erase(argv[p + 1].parse().expect("mode 1, 2 or 3"));
        return Ok(());
    }
    if let Some(p) = argv.iter().position(|a| a == "--macro-name") {
        let index = argv.get(p + 2).and_then(|a| a.parse::<u8>().ok()).unwrap_or(0);
        macro_name(&argv[p + 1], index);
        return Ok(());
    }
    if let Some(p) = argv.iter().position(|a| a == "--base-read") {
        let n = |i: usize| -> u8 {
            let t = &argv[i];
            u8::from_str_radix(t.trim_start_matches("0x"), 16).expect("hex")
        };
        base_read(n(p + 1), n(p + 2));
        return Ok(());
    }
    if std::env::args().any(|a| a == "--base-clear") {
        base_clear();
        return Ok(());
    }
    if std::env::args().any(|a| a == "--base-dump") {
        base_dump();
        return Ok(());
    }
    if std::env::args().any(|a| a == "--macro-import") {
        macro_import();
        return Ok(());
    }
    if std::env::args().any(|a| a == "--macro-dump") {
        macro_dump();
        return Ok(());
    }
    if std::env::args().any(|a| a == "--macro-list") {
        macro_list();
        return Ok(());
    }
    if std::env::args().any(|a| a == "--probe") {
        probe();
        return Ok(());
    }

    // The window **does not scroll**: it is born the size of its content.
    //
    // The width comes from the layout itself -- the keyboard drawing in
    // manufacturer pixels, times the scale, plus the side panel. The height is
    // the side panel's, which is taller than the keyboard. Since there is no
    // scrolling, the minimum is the same as the default: shrinking would only
    // clip.
    let (_, _, w, _) = layout::extent();
    let width = w * app::SCALE + app::PANEL_WIDTH + 30.0;
    // Height: the top bar, the drawn keyboard, the bottom columns and the apply
    // bar. With room to spare for the function section, which grows when a key
    // is selected.
    let height = 620.0;
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            // app_id becomes WM_CLASS on X11, which is how i3 identifies the
            // window for the float rule.
            .with_app_id("km780")
            .with_inner_size([width, height])
            .with_min_inner_size([width, height]),
        ..Default::default()
    };
    eframe::run_native(
        "KM780 RGB",
        options,
        Box::new(|cc| Ok(Box::new(app::App::new(cc)))),
    )
}
