//! The single screen of v1: read the keyboard, edit a local buffer, apply.
//!
//! The design follows the central requirement (`docs/29`): **nothing is
//! written to the keyboard before `Apply`**. Clicking a key and choosing a
//! color only touches the edit buffer; the bottom bar says how many keys are
//! dirty.

use std::collections::HashSet;
use std::sync::mpsc::{self, Receiver, TryRecvError};

use eframe::egui;
use egui::{Color32, Pos2, Rect, Sense, Stroke, Vec2};

use crate::assign::{self, OFF_FUNCTIONS};
use crate::effects;
use crate::layout;
use crate::profiles;
use crate::proto::{self, Channels, Keyboard};
use crate::slots::{self, NSLOTS};

/// How many screen pixels per pixel of the manufacturer's drawing.
///
/// `layout.rs` comes in G.SKILL pixels, where a normal key is 19-20 a side.
/// 1.9 puts the key at ~37 px, readable without a magnifier.
pub const SCALE: f32 = 1.9;
/// Width of the side panel, in screen pixels.
pub const PANEL_WIDTH: f32 = 270.0;
/// Gap between neighbouring keys, in screen pixels.
const GAP: f32 = 2.0;

/// Everything the app reads and edits from a mode: the colors and the mode
/// payload.
///
/// The two travel together because `Apply` is a single action -- the user
/// should not need to know that color goes over 0x09 and brightness over 0x02.
#[derive(Clone, PartialEq)]
struct State {
    colors: Vec<[u8; 3]>,
    /// The 264 bytes of the mode payload, as they came from 0x82.
    ///
    /// Kept **whole** on purpose: only bytes 10 and 11 are edited and all the
    /// rest travels intact through the round-trip. That is what makes it
    /// possible to touch the known without understanding the rest (`docs/29`,
    /// rule 3).
    info: Vec<u8>,
}

impl State {
    fn updated() -> State {
        State { colors: vec![[0, 0, 0]; NSLOTS], info: Vec::new() }
    }
    fn effect(&self) -> u8 {
        self.info.get(proto::OFF_EFFECT).copied().unwrap_or(0)
    }
    fn brightness(&self) -> u8 {
        self.info.get(proto::OFF_BRIGHTNESS).copied().unwrap_or(0)
    }
    fn has_info(&self) -> bool {
        self.info.len() == proto::PKT
    }
}

pub struct App {
    /// Device name of the last successful connection, just for the header.
    /// The device is **not** held open between operations -- see `with_keyboard`.
    device: Option<String>,
    /// Mode read with 0x81. Re-read on every `Reload`: the M1/M2/M3 keys switch
    /// the mode inside the keyboard, without telling the host.
    mode: u8,
    /// What the keyboard has today -- the source of truth.
    stored: State,
    /// The editable copy.
    buffer: State,
    /// State previous to the last `Apply`, for the undo.
    undo: Option<State>,
    selection: HashSet<usize>,
    color: [u8; 3],
    /// The color's hex as editable text. Kept apart from `color` so the user
    /// can type partially without the color jumping at every keystroke.
    color_hex: String,
    /// The function code typed in hex, for what is not in the catalog.
    function_hex: String,
    /// Source key of the "copy from".
    copy_source: String,
    /// Index of the range chosen in the first function combo box.
    function_group: usize,
    /// Function code chosen in the remap UI. F13 by default: it is the start of
    /// the free-key space (`docs/28`).
    function: u8,
    /// Name typed in the profiles box.
    profile_name: String,
    status: String,
    error: Option<String>,
    /// The USB operation in flight, if any. Only one at a time: they all come
    /// from a click, and the button is disabled while one runs.
    pending: Option<Receiver<Result<Reply, String>>>,
    /// Copy of the context, so the thread can wake the UI when it finishes.
    ctx: egui::Context,
}

/// What a USB task returns to the UI.
enum Reply {
    Read {
        device: String,
        mode: u8,
        info: Vec<u8>,
        colors: Channels,
    },
    Written {
        device: String,
        mode: u8,
        n: usize,
        color_dirty: bool,
        mode_dirty: bool,
        remaps: usize,
        updated: State,
    },
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut app = App {
            device: None,
            mode: 1,
            stored: State::updated(),
            buffer: State::updated(),
            undo: None,
            selection: HashSet::new(),
            color: [255, 0, 0],
            color_hex: "FF0000".into(),
            function_hex: "68".into(),
            copy_source: String::new(),
            // 1 = "Free keys": it is the range `docs/28` recommends, and the one
            // the user wants in 9 cases out of 10.
            function_group: 1,
            function: 0x68,
            profile_name: String::new(),
            status: String::new(),
            error: None,
            pending: None,
            ctx: cc.egui_ctx.clone(),
        };
        app.refresh();
        app
    }

    /// Runs a USB operation **off the UI thread**.
    ///
    /// The window must not freeze while the keyboard answers: a `Reload` is
    /// ~250 ms and an `Apply` ~160 ms, because of the mandatory timing in
    /// `docs/09` §2 (10 ms per transaction, 20 ms between channels, +100 ms
    /// after the third). That time belongs to the protocol, not to our
    /// implementation, so it cannot be shortened -- only moved out of the way.
    ///
    /// Each task **opens and closes** the device. Not a detail: measured on
    /// this machine on 2026-10-17, while interface 1 is claimed it leaves
    /// `usbhid` and goes to `usbfs` --
    ///
    ///     3-5:1.1 -> usbfs      (app holding)
    ///     3-5:1.1 -> usbhid     (app closed)
    ///
    /// and it is precisely interface 1 that declares Mouse, Consumer Control
    /// and the keyboard reports 0x08/0x09 (`docs/29`). Holding the device for
    /// the window's lifetime would take the media keys away from the user.
    /// Opening per operation costs an enumeration of a few milliseconds and
    /// gives the driver back between clicks.
    fn task(
        &mut self,
        f: impl FnOnce(&Keyboard) -> Result<Reply, String> + Send + 'static,
    ) {
        if self.pending.is_some() {
            return; // one is already running; the button is disabled anyway
        }
        let (tx, rx) = mpsc::channel();
        let ctx = self.ctx.clone();
        std::thread::spawn(move || {
            let r = Keyboard::open_device().and_then(|kb| f(&kb));
            let _ = tx.send(r);
            ctx.request_repaint(); // wake the UI, which may be idle
        });
        self.pending = Some(rx);
    }

    /// Collects the task's result, if it has arrived, and applies it to the state.
    fn collect(&mut self) {
        let got = match self.pending.as_ref().map(|rx| rx.try_recv()) {
            None | Some(Err(TryRecvError::Empty)) => return,
            Some(Err(TryRecvError::Disconnected)) => {
                self.pending = None;
                self.error = Some("the operation ended with no response".into());
                return;
            }
            Some(Ok(r)) => r,
        };
        self.pending = None;

        let r = match got {
            Ok(r) => r,
            Err(e) => {
                self.error = Some(e);
                return;
            }
        };
        self.error = None;
        match r {
            Reply::Read { device, mode, info, colors } => {
                self.device = Some(device);
                self.mode = mode;
                self.stored = State {
                    colors: (0..NSLOTS)
                        .map(|i| [colors[0][i], colors[1][i], colors[2][i]])
                        .collect(),
                    info,
                };
                self.buffer = self.stored.clone();
                self.selection.clear();
                self.status = format!("read from mode {mode}");
            }
            Reply::Written {
                device,
                mode,
                n,
                color_dirty,
                mode_dirty,
                remaps,
                updated,
            } => {
                self.device = Some(device);
                self.undo = Some(self.stored.clone());
                self.stored = updated;
                self.status = match (color_dirty, mode_dirty) {
                    (true, true) => format!("{n} key(s) and mode {mode} written"),
                    (true, false) => format!("{n} key(s) written to mode {mode}"),
                    _ => format!("mode {mode} written"),
                };
                if remaps > 0 {
                    self.status += &format!("; {remaps} function(s) written to all 3 modes");
                }
            }
        }
    }

    /// Switches the active mode on the keyboard and re-reads.
    ///
    /// Writing here does not break "nothing before Apply": `Apply` is about
    /// **edits**, and switching mode is navigation -- the same as the user
    /// pressing M1/M2/M3 on the keyboard itself. But since the following read
    /// replaces the buffer, pending edits would die: that is why the switch is
    /// refused while anything is pending.
    fn switch_mode(&mut self, updated: u8) {
        if updated == self.mode {
            return;
        }
        if self.pending_changes() > 0 {
            self.status = format!(
                "{} pending change(s) on mode {} \u{2014} Apply or Discard first",
                self.pending_changes(),
                self.mode
            );
            return;
        }
        self.status = format!("switching to mode {updated}\u{2026}");
        self.task(move |kb| {
            kb.select_mode(updated)?;
            // Re-read the active one instead of trusting what was written: it is
            // the same rule 4 of `docs/29`, and here it also confirms it took.
            let mode = kb.active_mode()?;
            if mode != updated {
                return Err(format!("asked for mode {updated}, the keyboard stayed on {mode}"));
            }
            let info = kb.read_mode(mode)?;
            let colors = kb.read_colors(mode)?;
            Ok(Reply::Read {
                device: kb.description.clone(),
                mode,
                info,
                colors,
            })
        });
    }

    /// Reads the whole keyboard: active mode, mode payload and the three channels.
    fn refresh(&mut self) {
        self.status = "reading\u{2026}".into();
        self.task(|kb| {
            let mode = kb.active_mode()?;
            if !(1..=3).contains(&mode) {
                return Err(format!("0x81 replied mode={mode}, out of 1..3"));
            }
            let info = kb.read_mode(mode)?;
            let colors = kb.read_colors(mode)?;
            Ok(Reply::Read {
                device: kb.description.clone(),
                mode,
                info,
                colors,
            })
        });
    }

    /// How many keys have a color different from the one read.
    fn dirty_keys(&self) -> usize {
        (0..NSLOTS)
            .filter(|&i| self.buffer.colors[i] != self.stored.colors[i])
            .count()
    }

    /// Whether brightness or effect were touched. They are the mode payload,
    /// which goes by a different command than color, but through the same `Apply`.
    fn mode_dirty(&self) -> bool {
        self.buffer.has_info() && self.buffer.info != self.stored.info
    }

    fn pending_changes(&self) -> usize {
        self.dirty_keys() + usize::from(self.mode_dirty())
    }

    /// Writes the buffer. Since color goes in three packets, one per channel,
    /// and there is no partial channel write (`docs/09` §6.4), any change
    /// resends the whole matrix on all three.
    fn apply(&mut self) {
        if self.pending_changes() == 0 {
            self.status = "nothing to apply".into();
            return;
        }
        let channels: Channels = [
            self.buffer.colors.iter().map(|c| c[0]).collect(),
            self.buffer.colors.iter().map(|c| c[1]).collect(),
            self.buffer.colors.iter().map(|c| c[2]).collect(),
        ];
        let n = self.dirty_keys();
        let color_dirty = n > 0;
        let mode_dirty = self.mode_dirty();
        let info = self.buffer.info.clone();
        let stored_info = self.stored.info.clone();
        let mode = self.mode;
        let updated = self.buffer.clone();

        self.status = "writing\u{2026}".into();
        self.task(move |kb| {
            // Re-read the mode before writing: the M1/M2/M3 keys switch the
            // mode inside the keyboard without telling the host (`docs/29`,
            // rule 4), and writing color to the wrong mode would paint the
            // wrong profile.
            let now = kb.active_mode()?;
            if now != mode {
                return Err(format!(
                    "the keyboard is on mode {now}, not the {mode} that was read \
                     -- somebody pressed M1/M2/M3. Reload before applying."
                ));
            }
            // The mode payload first: if the user turned the effect off to see
            // the static color, it is the color that should arrive last.
            if mode_dirty {
                kb.write_mode(mode, &info)?;
            }
            if color_dirty {
                kb.write_colors(mode, &channels)?;
            }
            // The other two modes receive only the function bytes that changed.
            // The same function the CLI's `--function` uses, which is what was
            // validated on the wire.
            let remaps = assign::propagate(kb, mode, &stored_info, &info)?.len();
            Ok(Reply::Written {
                device: kb.description.clone(),
                mode,
                n,
                color_dirty,
                mode_dirty,
                remaps,
                updated,
            })
        });
    }

    fn discard(&mut self) {
        self.refresh();
        self.status = "buffer discarded, re-read from the keyboard".into();
    }

    fn undo_apply(&mut self) {
        if let Some(previous_value) = self.undo.take() {
            self.buffer = previous_value;
            self.apply();
            self.status = "undone".into();
        }
    }

    fn paint(&mut self, color: [u8; 3]) {
        if self.selection.is_empty() {
            self.status = "no keys selected".into();
            return;
        }
        for &i in &self.selection {
            self.buffer.colors[i] = color;
        }
        self.status = format!("{} key(s) painted in the buffer", self.selection.len());
    }
}

fn to_color32(c: [u8; 3]) -> Color32 {
    Color32::from_rgb(c[0], c[1], c[2])
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.collect();
        let busy = self.pending.is_some();

        // ---- top: device identity and state ------------------------------
        // Only what the keyboard **is**, or what navigates between its
        // profiles, goes here. What edits lives in the inspector, on the right.
        // Mixing the two is what made the top bar confusing.
        egui::Panel::top("top").show(ui, |ui| {
            ui.horizontal(|ui| {
                match &self.device {
                    Some(name) => ui.strong(name),
                    None => ui.colored_label(WARN, "no keyboard"),
                };
                ui.separator();
                ui.label("mode");
                for m in 1..=3u8 {
                    if ui
                        .add_enabled(
                            !busy,
                            egui::Button::new(format!("M{m}")).selected(self.mode == m),
                        )
                        .on_hover_text("switches the mode on the keyboard, like M1/M2/M3 do")
                        .clicked()
                    {
                        self.switch_mode(m);
                    }
                }
                ui.separator();
                if ui
                    .add_enabled(!busy, egui::Button::new("Reload"))
                    .clicked()
                {
                    self.refresh();
                }
                if busy {
                    ui.spinner();
                }
            });

            ui.horizontal(|ui| {
                let has_info = self.buffer.has_info();

                let mut brightness = self.buffer.brightness();
                if ui
                    .add_enabled(
                        has_info && !busy,
                        egui::Slider::new(&mut brightness, 0..=4)
                            .show_value(false)
                            .text("brightness"),
                    )
                    .on_hover_text("five steps, 0 to 4 \u{2014} the same the keyboard's own key cycles")
                    .changed()
                {
                    self.buffer.info[proto::OFF_BRIGHTNESS] = brightness;
                }

                ui.separator();
                let current = self.buffer.effect();
                let mut chosen = current;
                egui::ComboBox::from_label("effect")
                    .selected_text(effects::name(current))
                    .show_ui(ui, |ui| {
                        for (v, name, _) in effects::CATALOG {
                            ui.selectable_value(&mut chosen, *v, *name);
                        }
                    });
                if chosen != current && has_info {
                    self.buffer.info[proto::OFF_EFFECT] = chosen;
                }
                // Ripple, Sonic, Changeling and Rain take their color from a
                // gradient already stored: the app only changes byte 10.
                // Without saying so, it looks like the color picker stopped
                // working.
                let note = effects::note(chosen);
                if !note.is_empty() {
                    ui.small(note);
                }

                ui.separator();
                self.profile_controls(ui, busy);
            });

            // The effect warning is a consequence, not decoration: with an
            // effect on, the static color does not show, and without this the
            // user thinks the app did not work.
            if self.buffer.effect() != 0 {
                ui.colored_label(
                    WARN,
                    format!(
                        "effect \"{}\" is on \u{2014} the animation hides the static colors",
                        effects::name(self.buffer.effect())
                    ),
                );
            }
            if let Some(e) = &self.error {
                ui.colored_label(ERROR, e);
            }
        });

        // ---- footer: the app's thesis ------------------------------------
        // Nothing goes to the keyboard before Apply, so the pending counter is
        // the most important piece of information on screen.
        egui::Panel::bottom("bar").show(ui, |ui| {
            let pending = self.pending_changes();
            ui.horizontal(|ui| {
                if pending == 0 {
                    ui.label("no pending changes");
                } else {
                    ui.colored_label(
                        PENDENTE,
                        egui::RichText::new(format!("\u{25cf} {pending} pending")).strong(),
                    );
                }
                ui.separator();
                let can = pending > 0 && !busy;
                if ui.add_enabled(can, egui::Button::new("Apply")).clicked() {
                    self.apply();
                }
                if ui.add_enabled(can, egui::Button::new("Discard")).clicked() {
                    self.discard();
                }
                if ui
                    .add_enabled(
                        self.undo.is_some() && !busy,
                        egui::Button::new("Undo last apply"),
                    )
                    .clicked()
                {
                    self.undo_apply();
                }
                ui.separator();
                ui.label(&self.status);
            });
        });

        // ---- inspector: always about the selection ------------------------
        egui::Panel::right("inspector")
            .default_size(PANEL_WIDTH)
            .show(ui, |ui| self.inspector(ui, busy));

        egui::CentralPanel::default().show(ui, |ui| {
            self.draw_keyboard(ui);
            ui.add_space(8.0);
            ui.small(
                "Click to select. Ctrl or Shift adds. Dragging over keys adds too.",
            );
            ui.small(
                "Some LEDs on this unit have the red component stuck: check \
                 colors by reading them back, not by eye.",
            );
        });
    }
}

/// Fixed palette of the inspector.
///
/// Eight swatches cover common use without opening the big picker, which now
/// lives in a popup. The order follows the color wheel so the row does not look
/// drawn at random.
static PALETTE: [[u8; 3]; 8] = [
    [255, 0, 0],
    [255, 96, 0],
    [255, 255, 0],
    [0, 255, 0],
    [0, 255, 255],
    [0, 0, 255],
    [255, 0, 255],
    [255, 255, 255],
];

const WARN: Color32 = Color32::from_rgb(220, 180, 80);
const ERROR: Color32 = Color32::from_rgb(230, 100, 100);
const PENDENTE: Color32 = Color32::from_rgb(240, 200, 100);

impl App {
    /// The right-hand panel: just one, and always about what is selected.
    ///
    /// There used to be a fixed "Color" on the right and a "Key function" in a
    /// column below, both useless with no selection -- half the screen sat
    /// inert. It is the inspector pattern ckb-next, iCUE and Chroma Studio use.
    fn inspector(&mut self, ui: &mut egui::Ui, busy: bool) {
        let mut sel: Vec<usize> = self.selection.iter().copied().collect();
        sel.sort_unstable();

        ui.heading("Selection");
        match sel.len() {
            0 => {
                ui.label("No keys selected.");
                ui.small("Click a key on the keyboard above.");
            }
            1 => {
                let slot = sel[0];
                ui.label(format!(
                    "{}  \u{2014} slot {slot}",
                    slots::SLOTS.get(slot).map(|n| slots::display_name(n)).unwrap_or("?")
                ));
            }
            n => {
                ui.label(format!("{n} keys"));
            }
        }
        ui.horizontal(|ui| {
            if ui.button("All").on_hover_text("every drawn key").clicked() {
                self.selection = layout::KEYS
                    .iter()
                    .filter_map(|t| slots::slot_of(t.name))
                    .collect();
            }
            if ui.button("None").clicked() {
                self.selection.clear();
            }
        });

        let has_sel = !sel.is_empty();

        // ---- color ---------------------------------------------------------
        ui.add_space(10.0);
        ui.separator();
        ui.label(egui::RichText::new("Color").strong());

        ui.horizontal(|ui| {
            // This button already opens the full picker in a popup. Leaving it
            // open on screen all the time cost ~230 px of panel.
            let mut c = to_color32(self.color);
            if egui::color_picker::color_edit_button_srgba(
                ui,
                &mut c,
                egui::color_picker::Alpha::Opaque,
            )
            .changed()
            {
                self.color = [c.r(), c.g(), c.b()];
                self.color_hex = hex_of(self.color);
            }
            let resp = ui.add(
                egui::TextEdit::singleline(&mut self.color_hex)
                    .desired_width(78.0)
                    .hint_text("RRGGBB"),
            );
            if resp.changed() {
                if let Some(c) = parse_hex(&self.color_hex) {
                    self.color = c;
                }
            }
            if resp.lost_focus() {
                self.color_hex = hex_of(self.color);
            }
        });

        ui.horizontal(|ui| {
            for c in PALETTE {
                let (rect, resp) =
                    ui.allocate_exact_size(Vec2::splat(20.0), Sense::click());
                ui.painter().rect_filled(rect, 3.0, to_color32(c));
                ui.painter().rect_stroke(
                    rect,
                    3.0,
                    Stroke::new(1.0, Color32::from_gray(90)),
                    egui::StrokeKind::Inside,
                );
                if resp.clicked() {
                    self.color = c;
                    self.color_hex = hex_of(c);
                }
            }
        });

        ui.horizontal(|ui| {
            if ui
                .add_enabled(has_sel, egui::Button::new("Paint"))
                .clicked()
            {
                let c = self.color;
                self.paint(c);
            }
            if ui
                .add_enabled(has_sel, egui::Button::new("Clear"))
                .clicked()
            {
                self.paint([0, 0, 0]);
            }
        });

        // ---- key function ---------------------------------------------------
        ui.add_space(10.0);
        ui.separator();
        ui.label(egui::RichText::new("Key function").strong());
        if !self.buffer.has_info() {
            ui.label("mode payload not read yet");
        } else if !has_sel {
            ui.small("select a key to see or change its function");
        } else {
            self.function_controls(ui, &sel, busy);
        }
    }
}

fn hex_of(c: [u8; 3]) -> String {
    format!("{:02X}{:02X}{:02X}", c[0], c[1], c[2])
}

fn parse_hex(t: &str) -> Option<[u8; 3]> {
    let t = t.trim().trim_start_matches('#');
    if t.len() != 6 {
        return None;
    }
    let n = u32::from_str_radix(t, 16).ok()?;
    Some([(n >> 16) as u8, (n >> 8) as u8, n as u8])
}

impl App {
    /// Key remapping: bytes 120-263 of the mode payload.
    ///
    /// It works for any of the 132 keys, and the remap lives in the keyboard --
    /// it survives a reboot and holds on another computer, with no process
    /// running (`docs/28`). Like everything here, it only goes to the wire on
    /// `Apply`.
    fn function_controls(&mut self, ui: &mut egui::Ui, sel: &[usize], busy: bool) {
        if let [slot] = sel {
            let o = OFF_FUNCTIONS + slot;
            let current = self.buffer.info[o];
            ui.label(format!("byte {o} = {}", assign::name(current)))
                .on_hover_text(assign::range_name(current));
        }

        ui.horizontal(|ui| {
            // Range and then item, instead of a flat list. The function byte
            // addresses far more than free keys (`docs/09` §5.2), and the
            // keyboard range alone has 106 entries -- in a single list the
            // popup ends up bigger than the window, and no amount of scrolling
            // saves that.
            let groups = assign::GROUPS;
            egui::ComboBox::from_id_salt("function_group")
                .selected_text(groups[self.function_group].0)
                .width(92.0)
                .show_ui(ui, |ui| {
                    for (i, (name, _)) in groups.iter().enumerate() {
                        if ui
                            .selectable_label(self.function_group == i, *name)
                            .clicked()
                        {
                            self.function_group = i;
                            // Switching range already selects its first item,
                            // so the combo box next to it never shows something
                            // that does not belong to the displayed range.
                            if let Some(first) = groups[i].1.first() {
                                self.function = first.code;
                                self.function_hex = format!("{:02X}", first.code);
                            }
                        }
                    }
                });
            egui::ComboBox::from_id_salt("function_item")
                .selected_text(assign::name(self.function))
                .width(128.0)
                .height(240.0)
                .show_ui(ui, |ui| {
                    for it in groups[self.function_group].1.iter() {
                        if ui
                            .selectable_label(self.function == it.code, it.name)
                            .clicked()
                        {
                            self.function = it.code;
                            self.function_hex = format!("{:02X}", it.code);
                        }
                    }
                });
            if ui
                .add_enabled(!busy, egui::Button::new("Assign"))
                .on_hover_text("goes to all three modes on Apply")
                .clicked()
            {
                let f = self.function;
                self.assign_to(sel, f);
            }
        });

        ui.horizontal(|ui| {
            // Any byte outside the catalog -- macro, text, or what has not been
            // named yet. No safety net: whoever types hex here knows what they
            // are doing, and the range shows up alongside to check against.
            ui.label("hex");
            let resp = ui.add(
                egui::TextEdit::singleline(&mut self.function_hex)
                    .desired_width(42.0)
                    .hint_text("68"),
            );
            if resp.changed() {
                if let Ok(v) = u8::from_str_radix(self.function_hex.trim(), 16) {
                    self.function = v;
                }
            }
            ui.small(assign::range_name(self.function));
        });

        ui.horizontal(|ui| {
            // "Copy from another key": takes the function the source key has
            // **now, in the buffer** -- not its factory HID. If the source has
            // already been remapped, it is the remapped value that travels,
            // which is what "copy this key" means.
            ui.label("copy from");
            let mut source = self.copy_source.clone();
            egui::ComboBox::from_id_salt("copy_source")
                .selected_text(if source.is_empty() { "\u{2014}" } else { &source })
                .width(96.0)
                .show_ui(ui, |ui| {
                    for t in layout::KEYS {
                        ui.selectable_value(
                            &mut source,
                            t.name.to_string(),
                            slots::display_name(t.name),
                        );
                    }
                });
            if source != self.copy_source {
                self.copy_source = source;
            }
            let from = slots::slot_of(&self.copy_source);
            if ui
                .add_enabled(from.is_some() && !busy, egui::Button::new("Copy"))
                .clicked()
            {
                let v = self.buffer.info[OFF_FUNCTIONS + from.unwrap()];
                self.function = v;
                self.function_hex = format!("{v:02X}");
                self.assign_to(sel, v);
            }
        });

        // Restore depends on having saved the original. With no record it
        // refuses instead of guessing: guessing a function code is worse than
        // doing nothing, and the factory table is in `docs/10`.
        if ui
            .add_enabled(!busy, egui::Button::new("Restore previous"))
            .on_hover_text(format!("from {}", assign::Record::path()))
            .clicked()
        {
            let record = assign::Record::load();
            let mode = self.mode;
            let mut done = 0;
            let mut missing: Vec<&str> = Vec::new();
            for &slot in sel {
                match record.previous(mode, slot) {
                    Some(v) => {
                        self.buffer.info[OFF_FUNCTIONS + slot] = v;
                        done += 1;
                    }
                    None => missing.push(slots::SLOTS.get(slot).copied().unwrap_or("?")),
                }
            }
            self.status = if missing.is_empty() {
                format!("{done} function(s) restored to the recorded value")
            } else {
                format!(
                    "{done} restored; nothing recorded for {}",
                    missing.join(", ")
                )
            };
        }
    }

    /// Puts a function code on the selected keys, **in the buffer**.
    fn assign_to(&mut self, sel: &[usize], code: u8) {
        for &slot in sel {
            self.buffer.info[OFF_FUNCTIONS + slot] = code;
        }
        self.status = format!(
            "{} key(s) set to {} in the buffer",
            sel.len(),
            assign::name(code)
        );
    }

    /// Profiles in a file of ours -- item 4 of `docs/29`.
    ///
    /// It sits at the top, next to the mode, because a profile is
    /// **navigation**, not key editing: it is the same family as the profile
    /// selector iCUE and Synapse put in the top bar. Loading writes nothing --
    /// it lands in the buffer and becomes pending, like every edit here.
    fn profile_controls(&mut self, ui: &mut egui::Ui, busy: bool) {
        ui.label("profile");
        let list = profiles::list();
        let mut chosen = self.profile_name.clone();
        egui::ComboBox::from_id_salt("profiles")
            .selected_text(if chosen.is_empty() { "\u{2014}" } else { &chosen })
            .width(110.0)
            .show_ui(ui, |ui| {
                for n in &list {
                    ui.selectable_value(&mut chosen, n.clone(), n);
                }
            });
        if chosen != self.profile_name {
            self.profile_name = chosen;
        }

        let name = self.profile_name.clone();
        if ui
            .add_enabled(list.contains(&name) && !busy, egui::Button::new("Load"))
            .clicked()
        {
            match profiles::load(&name) {
                Ok(p) => {
                    self.buffer.colors = p.colors;
                    if self.buffer.has_info() {
                        self.buffer.info[proto::OFF_BRIGHTNESS] = p.brightness;
                        self.buffer.info[proto::OFF_EFFECT] = p.effect;
                    }
                    self.status =
                        format!("profile \"{name}\" loaded into the buffer \u{2014} use Apply");
                }
                Err(e) => self.error = Some(e),
            }
        }

        ui.add(
            egui::TextEdit::singleline(&mut self.profile_name)
                .desired_width(90.0)
                .hint_text("name"),
        );
        if ui
            .add_enabled(!busy, egui::Button::new("Save"))
            .on_hover_text(format!(
                "colors, brightness and effect \u{2014} not the key functions. In {}",
                profiles::path()
            ))
            .clicked()
        {
            let p = profiles::Profile {
                colors: self.buffer.colors.clone(),
                brightness: self.buffer.brightness(),
                effect: self.buffer.effect(),
            };
            match profiles::save(&self.profile_name, &p) {
                Ok(file) => self.status = format!("profile saved to {file}"),
                Err(e) => self.error = Some(e),
            }
        }
    }

    fn draw_keyboard(&mut self, ui: &mut egui::Ui) {
        let (x0, y0, lu, lv) = layout::extent();
        let size = Vec2::new(lu * SCALE + 8.0, lv * SCALE + 8.0);
        let (response, painter) = ui.allocate_painter(size, Sense::hover());
        let base = response.rect.min + Vec2::splat(4.0);

        let additive = ui.input(|i| i.modifiers.ctrl || i.modifiers.shift);
        let mut clicks: Vec<usize> = Vec::new();
        let mut drags: Vec<usize> = Vec::new();

        for t in layout::KEYS {
            let Some(slot) = slots::slot_of(t.name) else { continue };
            let corner = Pos2::new(
                base.x + (t.x - x0) * SCALE,
                base.y + (t.y - y0) * SCALE,
            );
            let r = Rect::from_min_size(
                corner,
                Vec2::new(t.w * SCALE - GAP, t.h * SCALE - GAP),
            );

            let id = ui.make_persistent_id(("key", slot));
            let mut resp = ui.interact(r, id, Sense::click_and_drag());
            if resp.clicked() {
                clicks.push(slot);
            }
            if resp.hovered() {
                let c = self.buffer.colors[slot];
                let f = self.buffer.info.get(OFF_FUNCTIONS + slot).copied();
                resp = resp.on_hover_text(match f {
                    Some(f) => format!(
                        "{}  slot {slot}\ncolor #{:02X}{:02X}{:02X}\nfunction {} ({})",
                        slots::display_name(t.name), c[0], c[1], c[2],
                        assign::name(f), assign::range_name(f)
                    ),
                    None => format!("{}  slot {slot}\ncolor #{:02X}{:02X}{:02X}",
                                    slots::display_name(t.name), c[0], c[1], c[2]),
                });
            }
            if resp.dragged() {
                drags.push(slot);
            }

            let color = to_color32(self.buffer.colors[slot]);
            // The raw color is nearly invisible in the dark; the key background
            // mixes in a grey so the key still reads when it is off.
            let fill = Color32::from_rgb(
                28 + (color.r() as u16 * 227 / 255) as u8,
                28 + (color.g() as u16 * 227 / 255) as u8,
                28 + (color.b() as u16 * 227 / 255) as u8,
            );
            painter.rect_filled(r, 4.0, fill);

            let dirty = self.buffer.colors[slot] != self.stored.colors[slot];
            let selected = self.selection.contains(&slot);
            let stroke_style = if selected {
                Stroke::new(2.5, Color32::from_rgb(120, 200, 255))
            } else if dirty {
                Stroke::new(2.0, Color32::from_rgb(240, 200, 100))
            } else if resp.hovered() {
                Stroke::new(1.5, Color32::from_gray(180))
            } else {
                Stroke::new(1.0, Color32::from_gray(70))
            };
            painter.rect_stroke(r, 4.0, stroke_style, egui::StrokeKind::Inside);

            // Light or dark text, depending on the background's luminance.
            let lum = 0.299 * fill.r() as f32 + 0.587 * fill.g() as f32
                + 0.114 * fill.b() as f32;
            let ink = if lum > 140.0 { Color32::BLACK } else { Color32::from_gray(225) };
            // The keys in the top row are short and wide; the normal ones are
            // square. The font follows, otherwise the label overflows.
            let label = slots::display_name(t.name);
            let font_size = (r.height() * 0.32)
                .min(r.width() / (label.chars().count().max(1) as f32 * 0.62))
                .clamp(6.5, 12.0);
            painter.text(
                r.center(),
                egui::Align2::CENTER_CENTER,
                label,
                egui::FontId::proportional(font_size),
                ink,
            );
        }

        if !clicks.is_empty() {
            if !additive {
                self.selection.clear();
            }
            for slot in clicks {
                if additive && self.selection.contains(&slot) {
                    self.selection.remove(&slot);
                } else {
                    self.selection.insert(slot);
                }
            }
        }
        for slot in drags {
            self.selection.insert(slot);
        }
    }
}
