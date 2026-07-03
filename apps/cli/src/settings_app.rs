//! Settings & history window (eframe/egui), launched as
//! `whisper-catch settings` — from the tray menu or the shell.

use anyhow::Result;
use eframe::egui;

use crate::{autostart, config, theme};
use wc_core::history;

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

#[derive(PartialEq)]
enum Tab {
    History,
    Settings,
}

pub fn run() -> Result<()> {
    let cfg = config::load().unwrap_or_default();
    let pref = cfg.theme.clone();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([560.0, 660.0])
            .with_min_inner_size([440.0, 420.0]),
        centered: true,
        ..Default::default()
    };
    eframe::run_native(
        "WhisprCatch",
        options,
        Box::new(move |cc| {
            theme::apply(&cc.egui_ctx, &pref);
            Ok(Box::new(App::new(cfg)) as Box<dyn eframe::App>)
        }),
    )
    .map_err(|e| anyhow::anyhow!("settings window failed: {e}"))
}

struct App {
    tab: Tab,
    cfg: config::Config,
    autostart_on: bool,
    entries: Vec<history::Entry>,
    totals: (u64, u64, f32),
    status: String,
    confirm_clear: bool,
    search: String,
}

impl App {
    fn new(cfg: config::Config) -> Self {
        let autostart_on = dirs::config_dir()
            .map(|d| d.join("autostart/whisper-catch.desktop").exists())
            .unwrap_or(false);
        Self {
            tab: Tab::History,
            cfg,
            autostart_on,
            entries: history::load(500).unwrap_or_default(),
            totals: history::totals(),
            status: String::new(),
            confirm_clear: false,
            search: String::new(),
        }
    }

    fn reload_history(&mut self) {
        self.entries = history::load(500).unwrap_or_default();
        self.totals = history::totals();
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("header")
            .frame(egui::Frame::side_top_panel(&ctx.style()).inner_margin(12.0))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("WhisprCatch")
                            .heading()
                            .color(theme::accent(ui)),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let (n, w, s) = self.totals;
                        ui.label(
                            egui::RichText::new(format!(
                                "{w} words · {n} utterances · {:.0} min spoken",
                                s / 60.0
                            ))
                            .weak()
                            .small(),
                        );
                    });
                });
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.tab, Tab::History, "History");
                    ui.selectable_value(&mut self.tab, Tab::Settings, "Settings");
                });
            });

        egui::CentralPanel::default()
            .frame(egui::Frame::central_panel(&ctx.style()).inner_margin(12.0))
            .show(ctx, |ui| match self.tab {
                Tab::History => self.history_tab(ui),
                Tab::Settings => self.settings_tab(ui),
            });
    }
}

impl App {
    fn history_tab(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.search)
                    .hint_text("Search transcriptions…")
                    .desired_width(220.0),
            );
            if ui.button("Reload").clicked() {
                self.reload_history();
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if self.confirm_clear {
                    if ui.button("Cancel").clicked() {
                        self.confirm_clear = false;
                    }
                    if ui
                        .button(egui::RichText::new("Delete all").color(ui.visuals().error_fg_color))
                        .clicked()
                    {
                        let _ = history::clear();
                        self.reload_history();
                        self.confirm_clear = false;
                    }
                } else if ui.button("Clear…").clicked() {
                    self.confirm_clear = true;
                }
            });
        });
        ui.add_space(6.0);

        let q = self.search.to_lowercase();
        let shown: Vec<&history::Entry> = self
            .entries
            .iter()
            .filter(|e| q.is_empty() || e.text.to_lowercase().contains(&q))
            .collect();

        if shown.is_empty() {
            ui.add_space(40.0);
            ui.vertical_centered(|ui| {
                ui.label(
                    egui::RichText::new(if self.entries.is_empty() {
                        "No transcriptions yet.\nHold the hotkey and speak — they'll show up here."
                    } else {
                        "Nothing matches your search."
                    })
                    .weak(),
                );
            });
            return;
        }

        let mut copy: Option<String> = None;
        egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
            for e in shown {
                theme::card(ui).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        let when = chrono::DateTime::from_timestamp(e.ts as i64, 0)
                            .map(|t| {
                                t.with_timezone(&chrono::Local)
                                    .format("%b %d · %H:%M")
                                    .to_string()
                            })
                            .unwrap_or_default();
                        ui.label(
                            egui::RichText::new(when)
                                .small()
                                .color(theme::accent(ui)),
                        );
                        ui.label(
                            egui::RichText::new(format!("{:.1}s spoken", e.dur_s))
                                .weak()
                                .small(),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.small_button("Copy").clicked() {
                                copy = Some(e.text.clone());
                            }
                        });
                    });
                    ui.add_space(2.0);
                    ui.label(&e.text);
                });
                ui.add_space(4.0);
            }
        });
        if let Some(text) = copy {
            ui.ctx().copy_text(text);
        }
    }

    fn settings_tab(&mut self, ui: &mut egui::Ui) {
        let mut changed_theme = false;

        theme::card(ui).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(egui::RichText::new("Dictation").strong());
            ui.add_space(6.0);
            egui::Grid::new("dictation")
                .num_columns(2)
                .spacing([24.0, 10.0])
                .show(ui, |ui| {
                    ui.label("Push-to-talk key");
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

                    ui.label("Live typing");
                    ui.checkbox(&mut self.cfg.streaming, "words appear as you speak");
                    ui.end_row();

                    ui.label("Recording indicator");
                    ui.checkbox(&mut self.cfg.overlay, "floating pill while dictating");
                    ui.end_row();

                    ui.label("Keep history");
                    ui.checkbox(&mut self.cfg.history, "");
                    ui.end_row();
                });
        });
        ui.add_space(8.0);

        theme::card(ui).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(egui::RichText::new("Application").strong());
            ui.add_space(6.0);
            egui::Grid::new("application")
                .num_columns(2)
                .spacing([24.0, 10.0])
                .show(ui, |ui| {
                    ui.label("Start on login");
                    ui.checkbox(&mut self.autostart_on, "");
                    ui.end_row();

                    ui.label("Theme");
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

                    ui.label("Model");
                    ui.label(
                        egui::RichText::new(
                            self.cfg
                                .model_dir
                                .clone()
                                .unwrap_or_else(|| {
                                    wc_core::models_dir().join("parakeet-tdt-0.6b-v2-int8")
                                })
                                .display()
                                .to_string(),
                        )
                        .weak()
                        .small(),
                    );
                    ui.end_row();
                });
        });

        if changed_theme {
            theme::apply(ui.ctx(), &self.cfg.theme);
        }

        ui.add_space(12.0);
        ui.horizontal(|ui| {
            if ui.button(egui::RichText::new("Save").strong()).clicked() {
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
                    self.status = "Saved. Key changes apply after the daemon restarts.".into();
                }
            }
            if !self.status.is_empty() {
                ui.label(egui::RichText::new(&self.status).weak().small());
            }
        });
    }
}
