//! The single screen of v1: read the keyboard, edit a local buffer, apply.
//!
//! The design follows the central requirement (`docs/29`): **nothing is written to the
//! keyboard before `Apply`**. Clicking a key and picking a color only touches the
//! editing buffer; the bottom bar says how many keys are dirty.

use std::collections::HashSet;
use std::sync::mpsc::{self, Receiver, TryRecvError};

use eframe::egui;
use egui::{Color32, Pos2, Rect, Sense, Stroke, Vec2};

use crate::assign::{self, Record, OFF_FUNCTIONS};
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
const FOLGA: f32 = 2.0;

/// Everything the app reads and edits of a mode: the colors and the mode payload.
///
/// The two travel together because `Apply` is a single action -- the user
/// should not need to know that color goes over 0x09 and brightness over 0x02.
#[derive(Clone, PartialEq)]
struct State {
    colors: Vec<[u8; 3]>,
    /// The 264 bytes of the mode payload, as they came from 0x82.
    ///
    /// Kept **whole** on purpose: only bytes 10 and 11 are edited and
    /// all the rest travels intact through the round-trip. That is what allows touching the
    /// known without understanding the rest (`docs/29`, rule 3).
    info: Vec<u8>,
}

impl State {
    fn new_payload() -> State {
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
    /// State before the last `Apply`, for the undo.
    undo: Option<State>,
    selection: HashSet<usize>,
    color: [u8; 3],
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
        new_payload: State,
    },
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut app = App {
            device: None,
            mode: 1,
            stored: State::new_payload(),
            buffer: State::new_payload(),
            undo: None,
            selection: HashSet::new(),
            color: [255, 0, 0],
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
    /// mandatory one from `docs/09` §2 (10 ms per transaction, 20 ms between channels,
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
    /// driver entre cliques.
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
                self.error = Some("the operation ended with no reply".into());
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
                self.status = format!("stored do mode {mode}");
            }
            Reply::Written {
                device,
                mode,
                n,
                color_dirty,
                mode_dirty,
                remaps,
                new_payload,
            } => {
                self.device = Some(device);
                self.undo = Some(self.stored.clone());
                self.stored = new_payload;
                self.status = match (color_dirty, mode_dirty) {
                    (true, true) => format!("{n} key(s) and mode {mode} written"),
                    (true, false) => format!("{n} key(s) written in mode {mode}"),
                    _ => format!("mode {mode} written"),
                };
                if remaps > 0 {
                    self.status += &format!("; {remaps} function(s) stored in the 3 modes");
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
    fn switch_mode(&mut self, new_payload: u8) {
        if new_payload == self.mode {
            return;
        }
        if self.pending_changes() > 0 {
            self.status = format!(
                "there are {} pending change(s) in mode {} -- Apply or Discard before switching",
                self.pending_changes(),
                self.mode
            );
            return;
        }
        self.status = format!("switching to mode {new_payload}...");
        self.task(move |kb| {
            kb.switch_mode(new_payload)?;
            // Re-read the active one instead of trusting what was written: it is
            // the same rule 4 of `docs/29`, and here it also confirms it took.
            let mode = kb.active_mode()?;
            if mode != new_payload {
                return Err(format!("mode {new_payload} was asked for, the keyboard stayed on {mode}"));
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
        self.status = "reading...".into();
        self.task(|kb| {
            let mode = kb.active_mode()?;
            if !(1..=3).contains(&mode) {
                return Err(format!("the 0x81 replied mode={mode}, outside 1..3"));
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
            self.status = "nada a apply".into();
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
        let new_payload = self.buffer.clone();

        self.status = "writing...".into();
        self.task(move |kb| {
            // Re-read the mode before writing: the M1/M2/M3 keys switch the
            // mode inside the keyboard without telling the host (`docs/29`, rule 4),
            // and writing color in the wrong mode would paint the wrong profile.
            let now = kb.active_mode()?;
            if now != mode {
                return Err(format!(
                    "the keyboard is on mode {now}, not on the {mode} that was read \
                     -- someone pressed M1/M2/M3. Reload before applying."
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
                new_payload,
            })
        });
    }

    fn discard(&mut self) {
        self.refresh();
        self.status = "buffer discarded, re-read from the keyboard".into();
    }

    fn undo_apply(&mut self) {
        if let Some(anterior) = self.undo.take() {
            self.buffer = anterior;
            self.apply();
            self.status = "undone".into();
        }
    }

    fn paint(&mut self, color: [u8; 3]) {
        if self.selection.is_empty() {
            self.status = "no key selected".into();
            return;
        }
        for &i in &self.selection {
            self.buffer.colors[i] = color;
        }
        self.status = format!("{} key(s) painted in the buffer", self.selection.len());
    }
}

fn cor32(c: [u8; 3]) -> Color32 {
    Color32::from_rgb(c[0], c[1], c[2])
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.collect();
        let busy = self.pending.is_some();

        egui::Panel::top("topo").show(ui, |ui| {
            ui.horizontal(|ui| {
                match &self.device {
                    Some(name) => {
                        ui.strong(name);
                    }
                    None => {
                        ui.colored_label(Color32::from_rgb(220, 120, 80), "no keyboard");
                    }
                }
                ui.separator();
                if ui
                    .add_enabled(!busy, egui::Button::new("Reler"))
                    .clicked()
                {
                    self.refresh();
                }
                if busy {
                    ui.spinner();
                }
            });

            ui.horizontal(|ui| {
                // The keyboard's three modes. There are three and they are its own: the
                // M1/M2/M3 keys do the same from outside (`docs/09` §4).
                ui.label("mode:");
                for m in 1..=3u8 {
                    let selected = self.mode == m;
                    if ui
                        .add_enabled(
                            !busy,
                            egui::Button::new(format!("M{m}")).selected(selected),
                        )
                        .clicked()
                    {
                        self.switch_mode(m);
                    }
                }

                ui.separator();
                // Brightness is five steps, 0..4 -- the same ones the keyboard's
                // brightness key walks through. It is not a percentage.
                let mut brightness = self.buffer.brightness();
                let has_info = self.buffer.has_info();
                if ui
                    .add_enabled(
                        has_info && !busy,
                        egui::Slider::new(&mut brightness, 0..=4)
                            .show_value(false)
                            .text("brightness"),
                    )
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
                // Para Ripple, Sonic, Changeling e Rain o app so troca o
                // byte 10: the animation comes out with the gradient already stored on the
                // keyboard. Without this warning it looks as if the color picker stopped
                // working.
                let note = effects::note(chosen);
                if !note.is_empty() {
                    ui.small(note);
                }
            });
            if self.buffer.effect() != 0 {
                ui.colored_label(
                    Color32::from_rgb(220, 180, 80),
                    format!(
                        "Mode {} has the effect \"{}\" on -- the static \
                         color is hidden by the animation.",
                        self.mode,
                        effects::name(self.buffer.effect())
                    ),
                );
            }
            if let Some(e) = &self.error {
                ui.colored_label(Color32::from_rgb(230, 100, 100), e);
            }
        });

        egui::Panel::bottom("barra").show(ui, |ui| {
            let dirty_keys = self.pending_changes();
            ui.horizontal(|ui| {
                let can = dirty_keys > 0 && !busy;
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
                if dirty_keys == 0 {
                    ui.label("nothing pending");
                } else {
                    ui.colored_label(
                        Color32::from_rgb(240, 200, 100),
                        format!("{dirty_keys} key(s) pending"),
                    );
                }
                if self.mode_dirty() {
                    ui.separator();
                    ui.colored_label(
                        Color32::from_rgb(240, 200, 100),
                        format!(
                            "brightness {} -> {}, effect {} -> {}",
                            self.stored.brightness(),
                            self.buffer.brightness(),
                            self.stored.effect(),
                            self.buffer.effect()
                        ),
                    );
                }
                ui.separator();
                ui.label(&self.status);
            });
        });

        egui::Panel::right("side").default_size(PANEL_WIDTH).show(ui, |ui| {
            ui.heading("Color");
            let mut c = cor32(self.color);
            if egui::color_picker::color_edit_button_srgba(
                ui,
                &mut c,
                egui::color_picker::Alpha::Opaque,
            )
            .changed()
            {
                self.color = [c.r(), c.g(), c.b()];
            }
            egui::color_picker::color_picker_color32(
                ui,
                &mut c,
                egui::color_picker::Alpha::Opaque,
            );
            self.color = [c.r(), c.g(), c.b()];
            ui.label(format!("#{:02X}{:02X}{:02X}", self.color[0], self.color[1], self.color[2]));

            ui.add_space(8.0);
            if ui.button("Paint selected").clicked() {
                let color = self.color;
                self.paint(color);
            }
            if ui.button("Clear selected").clicked() {
                self.paint([0, 0, 0]);
            }

            ui.add_space(8.0);
            ui.horizontal(|ui| {
                // Only the keys that are drawn. The 22 slots with no key are left
                // out on purpose: the button says "All" and the user expects
                // the keyboard they are looking at. For the others, the CLI accepts the
                // numero do slot -- ver `docs/30`.
                if ui.button("Tudo").clicked() {
                    self.selection = layout::KEYS
                        .iter()
                        .filter_map(|t| slots::slot_of(t.name))
                        .collect();
                }
                if ui.button("Nada").clicked() {
                    self.selection.clear();
                }
            });
            ui.label(format!("{} selected(s)", self.selection.len()));

        });

        egui::CentralPanel::default().show(ui, |ui| {
            self.draw_keyboard(ui);

            // What is not color lives **below the keyboard**, in columns: there is
            // width to spare there and the side panel stays short. The color
            // picker stays beside it, which is where it serves best -- it sits
            // close to the keys it will paint.
            ui.add_space(8.0);
            ui.separator();
            ui.columns(2, |col| {
                col[0].heading("Key function");
                if self.buffer.has_info() {
                    self.function_controls(&mut col[0]);
                } else {
                    col[0].label("no mode payload read");
                }

                col[1].heading("Profiles");
                self.profile_controls(&mut col[1]);
            });

            ui.add_space(6.0);
            ui.small(
                "Click selects. Ctrl or Shift adds to the selection. Dragging \
                 over the keys also adds.  This unit has LEDs with the \
                 R component stuck (docs/27 §10): check the color by the \
                 re-read, not by eye.",
            );
        });
    }
}

impl App {
    /// Key remapping: bytes 120-263 of the mode payload.
    ///
    /// It works for any of the 132 keys, and the remap lives in the keyboard --
    /// keyboard -- it survives a reboot and holds on another computer, with no process
    /// running (`docs/28`). Like everything here, it only goes to the wire on `Apply`.
    fn function_controls(&mut self, ui: &mut egui::Ui) {
        let sel: Vec<usize> = {
            let mut v: Vec<usize> = self.selection.iter().copied().collect();
            v.sort_unstable();
            v
        };
        if sel.is_empty() {
            ui.small("select a key to see or change its function");
            return;
        }

        if let [slot] = sel[..] {
            let o = OFF_FUNCTIONS + slot;
            let current = self.buffer.info[o];
            let key_name = slots::SLOTS.get(slot).copied().unwrap_or("?");
            ui.label(format!(
                "{key_name}: byte {o} = {} ({})",
                assign::name(current),
                assign::range_name(current)
            ));
        } else {
            ui.label(format!("{} keys selected", sel.len()));
        }

        egui::ComboBox::from_label("nova function")
            .selected_text(assign::name(self.function))
            .show_ui(ui, |ui| {
                for c in assign::free_codes() {
                    ui.selectable_value(&mut self.function, c, assign::name(c));
                }
            });

        if ui.button("Assign to the selected").clicked() {
            let f = self.function;
            for &slot in &sel {
                self.buffer.info[OFF_FUNCTIONS + slot] = f;
            }
            self.status = format!(
                "{} key(s) become {} in the buffer -- it goes to the 3 modes on Apply",
                sel.len(),
                assign::name(f)
            );
        }

        // Restore depends on having stored the original. With no log it
        // refuses instead of guessing: guessing a function code is worse than doing
        // nothing, and the factory table is in `docs/10`.
        if ui.button("Restore previous function").clicked() {
            let record = Record::load();
            let mode = self.mode;
            let mut done = 0;
            let mut no_key: Vec<&str> = Vec::new();
            for &slot in &sel {
                match record.previous(mode, slot) {
                    Some(v) => {
                        self.buffer.info[OFF_FUNCTIONS + slot] = v;
                        done += 1;
                    }
                    None => no_key.push(slots::SLOTS.get(slot).copied().unwrap_or("?")),
                }
            }
            self.status = if no_key.is_empty() {
                format!("{done} function(s) went back to the stored value")
            } else {
                format!(
                    "{done} restored; no stored value for {} -- \
                     the factory code is in docs/10",
                    no_key.join(", ")
                )
            };
        }
        ui.small(format!("log: {}", Record::path()));
    }

    /// Profiles in a file of ours -- item 4 of `docs/29`.
    ///
    /// Saving takes a snapshot of the **buffer**, not of the keyboard: what is on
    /// the screen is what goes to the file. Loading does the opposite and **writes
    /// nothing** -- it lands in the buffer and becomes a pending change, like every edit
    /// here.
    fn profile_controls(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.profile_name)
                    .hint_text("name")
                    .desired_width(120.0),
            );
            if ui.button("Save").clicked() {
                let p = profiles::Profile {
                    colors: self.buffer.colors.clone(),
                    brightness: self.buffer.brightness(),
                    effect: self.buffer.effect(),
                };
                match profiles::save(&self.profile_name, &p) {
                    Ok(path) => self.status = format!("profile salvo em {path}"),
                    Err(e) => self.error = Some(e),
                }
            }
        });

        // A combo, and not one row per profile: the window does not scroll, so a
        // panel that grows with the number of files would end up cut off.
        let list = profiles::list();
        if list.is_empty() {
            ui.small("no profile saved yet");
        } else {
            ui.horizontal(|ui| {
                let mut chosen = self.profile_name.clone();
                egui::ComboBox::from_id_salt("profiles")
                    .selected_text(if chosen.is_empty() {
                        "pick one".to_string()
                    } else {
                        chosen.clone()
                    })
                    .width(140.0)
                    .show_ui(ui, |ui| {
                        for n in &list {
                            ui.selectable_value(&mut chosen, n.clone(), n);
                        }
                    });
                if chosen != self.profile_name {
                    self.profile_name = chosen;
                }
                let name = self.profile_name.clone();
                let can = list.contains(&name);
                if ui.add_enabled(can, egui::Button::new("Carregar")).clicked() {
                    match profiles::load(&name) {
                        Ok(p) => {
                            self.buffer.colors = p.colors;
                            if self.buffer.has_info() {
                                self.buffer.info[proto::OFF_BRIGHTNESS] = p.brightness;
                                self.buffer.info[proto::OFF_EFFECT] = p.effect;
                            }
                            self.status = format!(
                                "profile \"{name}\" loaded into the buffer -- \
                                 nothing was written; use Apply"
                            );
                        }
                        Err(e) => self.error = Some(e),
                    }
                }
            });
        }
        ui.small("a profile remembers color, brightness and effect -- not the keys' function");
        ui.small(format!("pasta: {}", profiles::path()));
    }

    fn draw_keyboard(&mut self, ui: &mut egui::Ui) {
        let (x0, y0, lu, lv) = layout::extent();
        let size = Vec2::new(lu * SCALE + 8.0, lv * SCALE + 8.0);
        let (resp, p) = ui.allocate_painter(size, Sense::hover());
        let base = resp.rect.min + Vec2::splat(4.0);

        let soma = ui.input(|i| i.modifiers.ctrl || i.modifiers.shift);
        let mut cliques: Vec<usize> = Vec::new();
        let mut drags: Vec<usize> = Vec::new();

        for t in layout::KEYS {
            let Some(slot) = slots::slot_of(t.name) else { continue };
            let canto = Pos2::new(
                base.x + (t.x - x0) * SCALE,
                base.y + (t.y - y0) * SCALE,
            );
            let r = Rect::from_min_size(
                canto,
                Vec2::new(t.w * SCALE - FOLGA, t.h * SCALE - FOLGA),
            );

            let id = ui.make_persistent_id(("key", slot));
            let mut resp = ui.interact(r, id, Sense::click_and_drag());
            if resp.clicked() {
                cliques.push(slot);
            }
            if resp.hovered() {
                let c = self.buffer.colors[slot];
                let f = self.buffer.info.get(OFF_FUNCTIONS + slot).copied();
                resp = resp.on_hover_text(match f {
                    Some(f) => format!(
                        "{}  slot {slot}\ncolor #{:02X}{:02X}{:02X}\nfunction {} ({})",
                        t.name, c[0], c[1], c[2],
                        assign::name(f), assign::range_name(f)
                    ),
                    None => format!("{}  slot {slot}\ncolor #{:02X}{:02X}{:02X}",
                                    t.name, c[0], c[1], c[2]),
                });
            }
            if resp.dragged() {
                drags.push(slot);
            }

            let color = cor32(self.buffer.colors[slot]);
            // The raw color is nearly invisible in the dark; the key background
            // mixes in a grey so the key still reads when it is off.
            let fill = Color32::from_rgb(
                28 + (color.r() as u16 * 227 / 255) as u8,
                28 + (color.g() as u16 * 227 / 255) as u8,
                28 + (color.b() as u16 * 227 / 255) as u8,
            );
            p.rect_filled(r, 4.0, fill);

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
            p.rect_stroke(r, 4.0, stroke_style, egui::StrokeKind::Inside);

            // Light or dark text, depending on the background's luminance.
            let lum = 0.299 * fill.r() as f32 + 0.587 * fill.g() as f32
                + 0.114 * fill.b() as f32;
            let tinta = if lum > 140.0 { Color32::BLACK } else { Color32::from_gray(225) };
            // The keys in the top row are short and wide; the normal ones are
            // square. The font follows, otherwise the label overflows.
            let fonte = (r.height() * 0.32)
                .min(r.width() / (t.name.chars().count().max(1) as f32 * 0.62))
                .clamp(6.5, 12.0);
            p.text(
                r.center(),
                egui::Align2::CENTER_CENTER,
                t.name,
                egui::FontId::proportional(fonte),
                tinta,
            );
        }

        if !cliques.is_empty() {
            if !soma {
                self.selection.clear();
            }
            for slot in cliques {
                if soma && self.selection.contains(&slot) {
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
