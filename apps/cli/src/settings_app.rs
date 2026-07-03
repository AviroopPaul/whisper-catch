//! Settings & history window (eframe/egui), launched as
//! `whisper-catch settings` — from the tray menu or the shell.
//!
//! Layout per docs/DESIGN.md §5: opens maximized, content in a centered
//! 760px column, header with accent title + stat chips, tabs with an
//! accent underline, history cards with hover affordance.

use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

use anyhow::Result;
use eframe::egui;
use egui_phosphor::regular as icons;
use wc_models::ModelId;

use crate::{autostart, config, theme};
use wc_core::history;

const MAX_COL_WIDTH: f32 = 760.0;
const GITHUB_URL: &str = "https://github.com/AviroopPaul/whisper-catch";
const SITE_URL: &str = "https://whisper-catch.vercel.app";

const KEYS: &[(&str, &str)] = &[
    ("ralt", "Right Alt"),
    ("lalt", "Left Alt"),
    ("rctrl", "Right Ctrl"),
    ("lctrl", "Left Ctrl"),
    ("super", "Super / Win"),
    ("f13", "F13"),
    ("scrolllock", "Scroll Lock"),
];

const THEMES: &[(&str, &str)] = &[
    ("system", "System"),
    ("light", "Light"),
    ("dark", "Dark"),
];

#[derive(PartialEq, Clone, Copy)]
enum Tab {
    History,
    Settings,
    About,
}

pub fn run() -> Result<()> {
    let cfg = config::load().unwrap_or_default();
    let pref = cfg.theme.clone();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_maximized(true)
            .with_inner_size([1080.0, 720.0])
            .with_min_inner_size([440.0, 420.0]),
        centered: true,
        ..Default::default()
    };
    eframe::run_native(
        "WhisprCatch",
        options,
        Box::new(move |cc| {
            theme::apply(&cc.egui_ctx, &pref);
            theme::install_icons(&cc.egui_ctx);
            Ok(Box::new(App::new(cfg)) as Box<dyn eframe::App>)
        }),
    )
    .map_err(|e| anyhow::anyhow!("settings window failed: {e}"))
}

/// Background model download driven from the Settings → Model card.
struct ModelDl {
    model: ModelId,
    rx: Receiver<DlMsg>,
    file: String,
    done: u64,
    total: u64,
    error: Option<String>,
}

enum DlMsg {
    Progress { file: String, done: u64, total: u64 },
    Finished,
    Failed(String),
}

struct App {
    tab: Tab,
    cfg: config::Config,
    autostart_on: bool,
    entries: Vec<history::Entry>,
    totals: (u64, u64, f32),
    status: String,
    saved_ok: bool,
    confirm_clear: bool,
    search: String,
    /// (entry timestamp, when copied) — drives the brief "Copied" flash.
    copied: Option<(u64, Instant)>,
    /// In-flight model download, if any (Settings → Model).
    dl: Option<ModelDl>,
}

impl App {
    fn new(cfg: config::Config) -> Self {
        let autostart_on = autostart::is_enabled();
        Self {
            tab: Tab::History,
            cfg,
            autostart_on,
            entries: history::load(500).unwrap_or_default(),
            totals: history::totals(),
            status: String::new(),
            saved_ok: false,
            confirm_clear: false,
            search: String::new(),
            copied: None,
            dl: None,
        }
    }

    /// The model currently selected in the config (defaults if unset/unknown).
    fn selected_model(&self) -> ModelId {
        ModelId::parse(&self.cfg.model)
    }

    /// Kick off a background download of `model` into the models dir.
    fn start_download(&mut self, model: ModelId, ctx: &egui::Context) {
        let (tx, rx) = mpsc::channel();
        self.dl = Some(ModelDl {
            model,
            rx,
            file: String::new(),
            done: 0,
            total: model.spec().total_size(),
            error: None,
        });
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let res = model.spec().ensure_with(&wc_core::models_dir(), &|f, d, t| {
                let _ = tx.send(DlMsg::Progress {
                    file: f.to_string(),
                    done: d,
                    total: t,
                });
                ctx.request_repaint();
            });
            let _ = tx.send(match res {
                Ok(_) => DlMsg::Finished,
                Err(e) => DlMsg::Failed(format!("{e:#}")),
            });
            ctx.request_repaint();
        });
    }

    /// Drains download progress messages; drops the job when it finishes.
    fn poll_download(&mut self) {
        let mut clear = false;
        if let Some(dl) = self.dl.as_mut() {
            while let Ok(msg) = dl.rx.try_recv() {
                match msg {
                    DlMsg::Progress { file, done, total } => {
                        dl.file = file;
                        dl.done = done;
                        dl.total = total;
                    }
                    DlMsg::Finished => clear = true,
                    DlMsg::Failed(e) => dl.error = Some(e),
                }
            }
        }
        if clear {
            self.dl = None;
        }
    }

    fn reload_history(&mut self) {
        self.entries = history::load(500).unwrap_or_default();
        self.totals = history::totals();
    }
}

/// Constrains content to a centered column so cards don't stretch to 1920px.
///
/// NOTE: must not go through `ui.horizontal(...)` — a horizontal row child Ui
/// only gets `interact_size.y` of height, so any `ScrollArea` inside collapses
/// to its 64px `min_scrolled_height`. Carve the column out of the full
/// available rect instead so children keep the panel's full height.
fn centered_col<R>(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    let full = ui.available_width();
    let w = full.min(MAX_COL_WIDTH);
    let pad = ((full - w) / 2.0).max(0.0);
    let mut rect = ui.available_rect_before_wrap();
    rect.min.x += pad;
    rect.max.x = rect.min.x + w;
    ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
        ui.set_width(w);
        add(ui)
    })
    .inner
}

/// Small accent-subtle pill: strong value + weak small label.
fn stat_chip(ui: &mut egui::Ui, value: &str, label: &str) {
    egui::Frame::default()
        .fill(theme::accent_subtle(ui))
        .corner_radius(egui::CornerRadius::same(14))
        .inner_margin(egui::Margin::symmetric(12, 5))
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.x = 5.0;
            ui.label(egui::RichText::new(value).strong());
            ui.label(egui::RichText::new(label).small().weak());
        });
}

/// Tab label; selected = accent text + 2px accent underline.
fn tab_button(ui: &mut egui::Ui, selected: bool, icon: &str, label: &str) -> bool {
    let a = theme::accent(ui);
    let text = format!("{icon}  {label}");
    let rich = if selected {
        egui::RichText::new(text).color(a).strong()
    } else {
        egui::RichText::new(text)
    };
    let resp = ui
        .add(egui::Label::new(rich).sense(egui::Sense::click()))
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    if selected {
        ui.painter().hline(
            resp.rect.x_range(),
            resp.rect.bottom() + 6.0,
            egui::Stroke::new(2.0, a),
        );
    }
    resp.clicked()
}

/// Section header inside a card: accent icon + 17/strong title.
fn section_title(ui: &mut egui::Ui, icon: &str, title: &str) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        ui.label(egui::RichText::new(icon).size(17.0).color(theme::accent(ui)));
        ui.label(egui::RichText::new(title).size(17.0).strong());
    });
}

/// Grid row label with a quiet leading icon.
fn setting_label(ui: &mut egui::Ui, icon: &str, text: &str) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        ui.label(egui::RichText::new(icon).weak());
        ui.label(text);
    });
}

/// Big glyph on a 96px accent-subtle plate + two lines — empty states, About.
fn glyph_plate(ui: &mut egui::Ui, icon: &str, size: f32) {
    let a = theme::accent(ui);
    let subtle = theme::accent_subtle(ui);
    let ring = theme::border(ui);
    let (rect, _) = ui.allocate_exact_size(egui::vec2(96.0, 96.0), egui::Sense::hover());
    let p = ui.painter();
    p.circle_filled(rect.center(), 48.0, subtle);
    p.circle_stroke(rect.center(), 48.0, egui::Stroke::new(1.0, ring));
    p.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        icon,
        egui::FontId::proportional(size),
        a,
    );
}

fn rel_time(ts: u64) -> String {
    let now = chrono::Utc::now().timestamp();
    let d = (now - ts as i64).max(0);
    if d < 60 {
        "just now".into()
    } else if d < 3_600 {
        format!("{}m ago", d / 60)
    } else if d < 86_400 {
        format!("{}h ago", d / 3_600)
    } else if d < 7 * 86_400 {
        format!("{}d ago", d / 86_400)
    } else {
        chrono::DateTime::from_timestamp(ts as i64, 0)
            .map(|t| {
                t.with_timezone(&chrono::Local)
                    .format("%b %d, %Y")
                    .to_string()
            })
            .unwrap_or_default()
    }
}

fn abs_time(ts: u64) -> String {
    chrono::DateTime::from_timestamp(ts as i64, 0)
        .map(|t| {
            t.with_timezone(&chrono::Local)
                .format("%b %d · %H:%M")
                .to_string()
        })
        .unwrap_or_default()
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_download();
        if self.dl.is_some() {
            ctx.request_repaint_after(Duration::from_millis(200));
        }
        egui::TopBottomPanel::top("header")
            .frame(egui::Frame::side_top_panel(&ctx.style()).inner_margin(egui::Margin {
                left: 24,
                right: 24,
                top: 16,
                bottom: 0,
            }))
            .show(ctx, |ui| {
                centered_col(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("WhisprCatch")
                                .heading()
                                .color(theme::accent(ui)),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.available_width() > 380.0 {
                                let (n, w, s) = self.totals;
                                stat_chip(ui, &format!("{:.0}", s / 60.0), "min spoken");
                                stat_chip(ui, &n.to_string(), "utterances");
                                stat_chip(ui, &w.to_string(), "words");
                            }
                        });
                    });
                    ui.add_space(14.0);
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 20.0;
                        if tab_button(
                            ui,
                            self.tab == Tab::History,
                            icons::CLOCK_COUNTER_CLOCKWISE,
                            "History",
                        ) {
                            self.tab = Tab::History;
                        }
                        if tab_button(ui, self.tab == Tab::Settings, icons::GEAR, "Settings") {
                            self.tab = Tab::Settings;
                        }
                        if tab_button(ui, self.tab == Tab::About, icons::INFO, "About") {
                            self.tab = Tab::About;
                        }
                    });
                    ui.add_space(10.0);
                });
            });

        egui::CentralPanel::default()
            .frame(egui::Frame::central_panel(&ctx.style()).inner_margin(egui::Margin {
                left: 24,
                right: 24,
                top: 16,
                bottom: 16,
            }))
            .show(ctx, |ui| {
                centered_col(ui, |ui| match self.tab {
                    Tab::History => self.history_tab(ui),
                    Tab::Settings => self.settings_tab(ui),
                    Tab::About => self.about_tab(ui),
                });
            });
    }
}

impl App {
    fn history_tab(&mut self, ui: &mut egui::Ui) {
        // Constrain the toolbar row's height: a bare `with_layout` here would
        // greedily take the panel's full remaining height and center the row.
        ui.allocate_ui_with_layout(
            egui::vec2(ui.available_width(), 30.0),
            egui::Layout::right_to_left(egui::Align::Center),
            |ui| {
            if self.confirm_clear {
                if ui.button("Cancel").clicked() {
                    self.confirm_clear = false;
                }
                if ui
                    .button(
                        egui::RichText::new(format!("{} Delete all", icons::TRASH))
                            .color(ui.visuals().error_fg_color),
                    )
                    .clicked()
                {
                    let _ = history::clear();
                    self.reload_history();
                    self.confirm_clear = false;
                }
            } else if ui
                .button(format!("{} Clear…", icons::TRASH))
                .clicked()
            {
                self.confirm_clear = true;
            }
            if ui
                .button(format!("{} Reload", icons::ARROWS_CLOCKWISE))
                .clicked()
            {
                self.reload_history();
            }
            let w = ui.available_width();
            ui.add_sized(
                [w, 30.0],
                egui::TextEdit::singleline(&mut self.search).hint_text(format!(
                    "{}  Search transcriptions…",
                    icons::MAGNIFYING_GLASS
                )),
            );
        });
        ui.add_space(10.0);

        let q = self.search.to_lowercase();
        let shown: Vec<&history::Entry> = self
            .entries
            .iter()
            .filter(|e| q.is_empty() || e.text.to_lowercase().contains(&q))
            .collect();

        if shown.is_empty() {
            ui.add_space((ui.available_height() * 0.26).clamp(24.0, 220.0));
            ui.vertical_centered(|ui| {
                if self.entries.is_empty() {
                    glyph_plate(ui, icons::MICROPHONE, 42.0);
                    ui.add_space(16.0);
                    ui.label(egui::RichText::new("No transcriptions yet").size(17.0).strong());
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new(
                            "Hold the hotkey and speak — they'll show up here.",
                        )
                        .weak(),
                    );
                } else {
                    glyph_plate(ui, icons::MAGNIFYING_GLASS, 42.0);
                    ui.add_space(16.0);
                    ui.label(
                        egui::RichText::new("Nothing matches your search")
                            .size(17.0)
                            .strong(),
                    );
                    ui.add_space(4.0);
                    ui.label(egui::RichText::new("Try a different phrase.").weak());
                }
            });
            return;
        }

        if !q.is_empty() {
            ui.label(
                egui::RichText::new(format!("{} of {}", shown.len(), self.entries.len()))
                    .small()
                    .weak(),
            );
            ui.add_space(4.0);
        }

        let mut copy: Option<(u64, String)> = None;
        egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
            for e in shown {
                let resp = theme::card(ui).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(rel_time(e.ts))
                                .small()
                                .strong()
                                .color(theme::accent(ui)),
                        );
                        ui.label(
                            egui::RichText::new(format!(
                                "{}  ·  {:.1}s spoken",
                                abs_time(e.ts),
                                e.dur_s
                            ))
                            .weak()
                            .small(),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let flash = self
                                .copied
                                .as_ref()
                                .is_some_and(|(ts, at)| {
                                    *ts == e.ts && at.elapsed() < Duration::from_millis(1500)
                                });
                            if flash {
                                ui.label(
                                    egui::RichText::new(format!("{} Copied", icons::CHECK))
                                        .small()
                                        .color(theme::success(ui)),
                                );
                                ui.ctx().request_repaint_after(Duration::from_millis(200));
                            } else if ui
                                .small_button(format!("{} Copy", icons::COPY))
                                .clicked()
                            {
                                copy = Some((e.ts, e.text.clone()));
                            }
                        });
                    });
                    ui.add_space(4.0);
                    ui.label(&e.text);
                });
                if resp.response.hovered() {
                    ui.painter().rect_stroke(
                        resp.response.rect,
                        egui::CornerRadius::same(12),
                        egui::Stroke::new(1.0, theme::border_strong(ui)),
                        egui::StrokeKind::Inside,
                    );
                }
                ui.add_space(8.0);
            }
        });
        if let Some((ts, text)) = copy {
            ui.ctx().copy_text(text);
            self.copied = Some((ts, Instant::now()));
        }
    }

    fn settings_tab(&mut self, ui: &mut egui::Ui) {
        let mut changed_theme = false;

        egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
            theme::card(ui).show(ui, |ui| {
                ui.set_width(ui.available_width());
                section_title(ui, icons::MICROPHONE, "Dictation");
                ui.add_space(8.0);
                egui::Grid::new("dictation")
                    .num_columns(2)
                    .spacing([24.0, 10.0])
                    .show(ui, |ui| {
                        setting_label(ui, icons::KEYBOARD, "Push-to-talk key");
                        let current = KEYS
                            .iter()
                            .find(|(k, _)| *k == self.cfg.key)
                            .map(|(_, l)| *l)
                            .unwrap_or(self.cfg.key.as_str());
                        egui::ComboBox::from_id_salt("key")
                            .selected_text(current)
                            .show_ui(ui, |ui| {
                                for (k, label) in KEYS {
                                    ui.selectable_value(&mut self.cfg.key, k.to_string(), *label);
                                }
                            });
                        ui.end_row();

                        setting_label(ui, icons::CURSOR_TEXT, "Live typing");
                        ui.checkbox(&mut self.cfg.streaming, "words appear as you speak");
                        ui.end_row();

                        setting_label(ui, icons::RECORD, "Recording indicator");
                        ui.checkbox(&mut self.cfg.overlay, "floating pill while dictating");
                        ui.end_row();

                        setting_label(ui, icons::CLOCK_COUNTER_CLOCKWISE, "Keep history");
                        ui.checkbox(&mut self.cfg.history, "log transcriptions locally");
                        ui.end_row();
                    });
            });
            ui.add_space(12.0);

            theme::card(ui).show(ui, |ui| {
                ui.set_width(ui.available_width());
                section_title(ui, icons::GEAR, "Application");
                ui.add_space(8.0);
                egui::Grid::new("application")
                    .num_columns(2)
                    .spacing([24.0, 10.0])
                    .show(ui, |ui| {
                        setting_label(ui, icons::ROCKET_LAUNCH, "Start on login");
                        ui.checkbox(&mut self.autostart_on, "");
                        ui.end_row();

                        setting_label(ui, icons::PALETTE, "Theme");
                        let current = THEMES
                            .iter()
                            .find(|(k, _)| *k == self.cfg.theme)
                            .map(|(_, l)| *l)
                            .unwrap_or("System");
                        egui::ComboBox::from_id_salt("theme")
                            .selected_text(current)
                            .show_ui(ui, |ui| {
                                for (k, label) in THEMES {
                                    if ui
                                        .selectable_value(&mut self.cfg.theme, k.to_string(), *label)
                                        .clicked()
                                    {
                                        changed_theme = true;
                                    }
                                }
                            });
                        ui.end_row();
                    });
            });
            ui.add_space(12.0);

            self.model_card(ui);

            ui.add_space(16.0);
            ui.horizontal(|ui| {
                if theme::accent_button(ui, format!("{}  Save", icons::FLOPPY_DISK)).clicked() {
                    let mut ok = true;
                    if let Err(e) = config::save(&self.cfg) {
                        self.status = format!("save failed: {e}");
                        ok = false;
                    }
                    let res = if self.autostart_on {
                        autostart::enable()
                    } else {
                        autostart::disable()
                    };
                    if let Err(e) = res {
                        self.status = format!("autostart failed: {e}");
                        ok = false;
                    }
                    if ok {
                        self.status =
                            "Saved. Model and key changes apply after the daemon restarts.".into();
                    }
                    self.saved_ok = ok;
                }
                if !self.status.is_empty() {
                    if self.saved_ok {
                        ui.label(
                            egui::RichText::new(icons::CHECK)
                                .small()
                                .color(theme::success(ui)),
                        );
                        ui.label(egui::RichText::new(&self.status).weak().small());
                    } else {
                        ui.label(
                            egui::RichText::new(&self.status)
                                .small()
                                .color(ui.visuals().error_fg_color),
                        );
                    }
                }
            });
        });

        if changed_theme {
            theme::apply(ui.ctx(), &self.cfg.theme);
        }
    }

    /// Speech-model picker with download-on-demand and progress.
    fn model_card(&mut self, ui: &mut egui::Ui) {
        let selected = self.selected_model();
        let complete = selected.spec().is_complete(&wc_core::models_dir());
        let downloading = self.dl.as_ref().map(|d| d.model) == Some(selected);
        let mut do_download: Option<ModelId> = None;

        theme::card(ui).show(ui, |ui| {
            ui.set_width(ui.available_width());
            section_title(ui, icons::CPU, "Speech model");
            ui.add_space(8.0);

            egui::Grid::new("model")
                .num_columns(2)
                .spacing([24.0, 10.0])
                .show(ui, |ui| {
                    setting_label(ui, icons::WAVEFORM, "Model");
                    let current = selected.label();
                    egui::ComboBox::from_id_salt("model")
                        .selected_text(current)
                        .show_ui(ui, |ui| {
                            for m in ModelId::ALL {
                                ui.selectable_value(
                                    &mut self.cfg.model,
                                    m.slug().to_string(),
                                    m.label(),
                                );
                            }
                        });
                    ui.end_row();
                });

            ui.add_space(6.0);
            ui.label(egui::RichText::new(selected.blurb()).weak());
            ui.add_space(2.0);
            ui.label(
                egui::RichText::new(format!(
                    "{}  ·  {} MB download",
                    selected.ram_hint(),
                    selected.download_mb()
                ))
                .small()
                .weak(),
            );

            ui.add_space(12.0);

            if downloading {
                let dl = self.dl.as_ref().unwrap();
                let frac = if dl.total > 0 {
                    dl.done as f32 / dl.total as f32
                } else {
                    0.0
                };
                ui.add(
                    egui::ProgressBar::new(frac)
                        .desired_height(10.0)
                        .fill(theme::accent(ui))
                        .animate(dl.error.is_none()),
                );
                ui.add_space(6.0);
                if let Some(e) = &dl.error {
                    ui.colored_label(ui.visuals().error_fg_color, e);
                } else {
                    ui.label(
                        egui::RichText::new(format!(
                            "{:.0}%  ·  {:.0} / {:.0} MB — {}",
                            frac * 100.0,
                            dl.done as f64 / 1e6,
                            dl.total as f64 / 1e6,
                            if dl.file.is_empty() { "preparing…" } else { &dl.file }
                        ))
                        .small()
                        .weak(),
                    );
                }
            } else if complete {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(format!("{} Downloaded", icons::CHECK))
                            .small()
                            .color(theme::success(ui)),
                    );
                    ui.label(
                        egui::RichText::new("· applies after the daemon restarts")
                            .small()
                            .weak(),
                    );
                });
            } else {
                ui.horizontal(|ui| {
                    if ui
                        .button(format!(
                            "{}  Download ({} MB)",
                            icons::DOWNLOAD_SIMPLE,
                            selected.download_mb()
                        ))
                        .clicked()
                    {
                        do_download = Some(selected);
                    }
                    ui.label(
                        egui::RichText::new("Not downloaded yet")
                            .small()
                            .weak(),
                    );
                });
            }
        });

        if let Some(m) = do_download {
            let ctx = ui.ctx().clone();
            self.start_download(m, &ctx);
        }
    }

    fn about_tab(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
            ui.add_space(48.0);
            ui.vertical_centered(|ui| {
                glyph_plate(ui, icons::WAVEFORM, 44.0);
                ui.add_space(16.0);
                ui.label(egui::RichText::new("WhisprCatch").size(26.0).strong());
                ui.add_space(2.0);
                ui.label(
                    egui::RichText::new(format!("v{}", env!("CARGO_PKG_VERSION")))
                        .monospace()
                        .weak(),
                );
                ui.add_space(16.0);
                ui.label("Push-to-talk dictation that runs entirely on your machine.");
                ui.add_space(2.0);
                ui.label(
                    egui::RichText::new(
                        "Hold a key, speak, release — text appears at your cursor.",
                    )
                    .weak(),
                );
                ui.add_space(20.0);
                ui.hyperlink_to(format!("{}  GitHub", icons::GITHUB_LOGO), GITHUB_URL);
                ui.add_space(4.0);
                ui.hyperlink_to(format!("{}  whisper-catch.vercel.app", icons::GLOBE), SITE_URL);
                ui.add_space(24.0);
                ui.label(
                    egui::RichText::new(format!(
                        "Config · {}",
                        config::config_path().display()
                    ))
                    .monospace()
                    .weak()
                    .small(),
                );
                ui.add_space(16.0);
                ui.label(
                    egui::RichText::new(
                        "MIT licensed · Built for people who think faster than they type.",
                    )
                    .weak()
                    .small(),
                );
            });
        });
    }
}
