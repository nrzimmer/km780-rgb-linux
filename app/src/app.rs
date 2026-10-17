//! The single screen of v1: read the keyboard, edit a local buffer, apply.
//!
//! The design follows the central requirement (`docs/29`): **nothing is written to the
//! keyboard before `Apply`**. Clicking a key and picking a color only touches the
//! editing buffer; the bottom bar says how many keys are dirty.

use std::collections::HashSet;

use eframe::egui;
use egui::{Color32, Pos2, Rect, Sense, Stroke, Vec2};

use crate::effects;
use crate::layout;
use crate::proto::{self, Channels, Keyboard};
use crate::slots::{self, NSLOTS};

/// Side of a 1u key, in pixels.
const UNIDADE: f32 = 38.0;
const FOLGA: f32 = 3.0;

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
    status: String,
    error: Option<String>,
    show_others: bool,
}

impl App {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let mut app = App {
            device: None,
            mode: 1,
            stored: State::new_payload(),
            buffer: State::new_payload(),
            undo: None,
            selection: HashSet::new(),
            color: [255, 0, 0],
            status: String::new(),
            error: None,
            show_others: false,
        };
        app.refresh();
        app
    }

    /// Opens the keyboard, runs `f`, and **closes**.
    ///
    /// The device is not left open between operations, and that is no detail:
    /// measured on this machine on 2026-10-17, while interface 1 is
    /// `usbhid` and goes to `usbfs` --
    ///
    ///     3-5:1.1 -> usbfs      (app holding)
    ///     3-5:1.1 -> usbhid     (app closed)
    ///
    /// and it is precisely interface 1 that declares Mouse, Consumer Control
    /// and the keyboard reports 0x08/0x09 (`docs/29`). Holding the device for
    /// life of the window would take the user's media keys away while the app
    /// was open. Opening per operation costs an enumeration of a few
    /// milliseconds and gives the driver back between clicks.
    fn with_keyboard<T>(&mut self, f: impl FnOnce(&Keyboard) -> Result<T, String>) -> Option<T> {
        match Keyboard::open_device() {
            Ok(kb) => {
                self.device = Some(kb.description.clone());
                match f(&kb) {
                    Ok(v) => {
                        self.error = None;
                        Some(v)
                    }
                    Err(e) => {
                        self.error = Some(e);
                        None
                    }
                }
            }
            Err(e) => {
                self.device = None;
                self.error = Some(e);
                None
            }
        }
    }

    /// Reads the whole keyboard: active mode, mode payload and the three channels.
    fn refresh(&mut self) {
        let r = self.with_keyboard(|kb| -> Result<(u8, Vec<u8>, Channels), String> {
            let mode = kb.active_mode()?;
            if !(1..=3).contains(&mode) {
                return Err(format!("the 0x81 replied mode={mode}, outside 1..3"));
            }
            let info = kb.read_mode(mode)?;
            let colors = kb.read_colors(mode)?;
            Ok((mode, info, colors))
        });
        if let Some((mode, info, colors)) = r {
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
        let mode = self.mode;
        // Re-read the mode before writing: the M1/M2/M3 keys switch the mode
        // inside the keyboard without telling the host (`docs/29`, rule 4), and writing
        // color in the wrong mode would paint the wrong profile.
        let ok = self.with_keyboard(|kb| {
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
            Ok(())
        });
        if ok.is_some() {
            self.undo = Some(self.stored.clone());
            self.stored = self.buffer.clone();
            self.status = match (color_dirty, mode_dirty) {
                (true, true) => format!("{n} key(s) and mode {mode} written"),
                (true, false) => format!("{n} key(s) written in mode {mode}"),
                _ => format!("brightness/effect written in mode {mode}"),
            };
        }
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
        egui::Panel::top("topo").show(ui, |ui| {
            ui.horizontal(|ui| {
                match &self.device {
                    Some(name) => {
                        ui.strong(name);
                        ui.label(format!("mode {}", self.mode));
                        ui.label(format!("brightness {}", self.stored.brightness()));
                        ui.label(format!("effect: {}", effects::name(self.stored.effect())));
                    }
                    None => {
                        ui.colored_label(Color32::from_rgb(220, 120, 80), "no keyboard");
                    }
                }
                ui.separator();
                if ui.button("Reler").clicked() {
                    self.refresh();
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
                let can = dirty_keys > 0;
                if ui.add_enabled(can, egui::Button::new("Apply")).clicked() {
                    self.apply();
                }
                if ui.add_enabled(can, egui::Button::new("Discard")).clicked() {
                    self.discard();
                }
                if ui
                    .add_enabled(
                        self.undo.is_some(),
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
                ui.separator();
                ui.label(&self.status);
            });
        });

        egui::Panel::right("side").default_size(240.0).show(ui, |ui| {
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

            ui.add_space(12.0);
            ui.separator();
            ui.heading("Mode");
            if self.buffer.has_info() {
                self.mode_controls(ui);
            } else {
                ui.label("no mode payload read");
            }

            ui.add_space(12.0);
            ui.collapsing("Outros slots", |ui| {
                ui.label(
                    "Named slots the drawing does not place: international \
                     keys, codes with no key in the US/ABNT layout, and the \
                     NumSep.",
                );
                self.show_others = true;
                let placed: HashSet<usize> = layout::KEYS
                    .iter()
                    .filter_map(|t| slots::slot_of(t.name))
                    .collect();
                for (i, name) in slots::SLOTS.iter().enumerate() {
                    if placed.contains(&i) {
                        continue;
                    }
                    let sel = self.selection.contains(&i);
                    let no_key = slots::NO_KEY.contains(&i);
                    let label = if no_key {
                        format!("{i:3} {name}  (no physical key)")
                    } else {
                        format!("{i:3} {name}")
                    };
                    if ui.selectable_label(sel, label).clicked() {
                        if sel {
                            self.selection.remove(&i);
                        } else {
                            self.selection.insert(i);
                        }
                    }
                }
            });

            ui.add_space(12.0);
            ui.small(
                "Click selects. Ctrl or Shift adds to the selection. Dragging \
                 over the keys also adds.",
            );
            ui.small(
                "This unit has LEDs with the R component stuck (docs/27 \
                 §10): check the color by the re-read, not by eye.",
            );
        });

        egui::CentralPanel::default().show(ui, |ui| {
            egui::ScrollArea::both().show(ui, |ui| self.draw_keyboard(ui));
        });
    }
}

impl App {
    /// Brightness and effect: bytes 11 and 10 of the mode payload.
    ///
    /// Both are **read-change-write** -- they edit one byte of the payload that
    /// came from the 0x82 and the rest travels intact. Like everything in the app, nothing goes to
    /// the keyboard before `Apply`.
    fn mode_controls(&mut self, ui: &mut egui::Ui) {
        let mut brightness = self.buffer.brightness();
        // Brightness is 0..4 (`docs/09`). It is not a percentage: it is five steps,
        // the same ones the keyboard's brightness key walks through.
        if ui
            .add(egui::Slider::new(&mut brightness, 0..=4).text("brightness"))
            .changed()
        {
            self.buffer.info[proto::OFF_BRIGHTNESS] = brightness;
        }

        let current = self.buffer.effect();
        let mut chosen = current;
        egui::ComboBox::from_label("effect")
            .selected_text(effects::name(current))
            .show_ui(ui, |ui| {
                for (v, name, _) in effects::CATALOG {
                    ui.selectable_value(&mut chosen, *v, *name);
                }
            });
        if chosen != current {
            self.buffer.info[proto::OFF_EFFECT] = chosen;
        }
        let note = effects::note(chosen);
        if !note.is_empty() {
            ui.small(note);
        }

        if self.mode_dirty() {
            ui.colored_label(
                Color32::from_rgb(240, 200, 100),
                format!(
                    "mode pending: brightness {} -> {}, effect {} -> {}",
                    self.stored.brightness(),
                    self.buffer.brightness(),
                    self.stored.effect(),
                    self.buffer.effect()
                ),
            );
        }
    }

    fn draw_keyboard(&mut self, ui: &mut egui::Ui) {
        let (lu, lv) = layout::extent();
        let size = Vec2::new(lu * UNIDADE + 8.0, lv * UNIDADE + 8.0);
        let (resp, p) = ui.allocate_painter(size, Sense::hover());
        let base = resp.rect.min + Vec2::splat(4.0);

        let soma = ui.input(|i| i.modifiers.ctrl || i.modifiers.shift);
        let mut cliques: Vec<usize> = Vec::new();
        let mut drags: Vec<usize> = Vec::new();

        for t in layout::KEYS {
            let Some(slot) = slots::slot_of(t.name) else { continue };
            let canto = Pos2::new(
                base.x + t.x * UNIDADE,
                base.y + (t.y + layout::Y_OFFSET) * UNIDADE,
            );
            let r = Rect::from_min_size(
                canto,
                Vec2::new(t.w * UNIDADE - FOLGA, t.h * UNIDADE - FOLGA),
            );

            let id = ui.make_persistent_id(("key", slot));
            let resp = ui.interact(r, id, Sense::click_and_drag());
            if resp.clicked() {
                cliques.push(slot);
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
            let fonte = if t.name.chars().count() > 4 { 9.0 } else { 11.0 };
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
