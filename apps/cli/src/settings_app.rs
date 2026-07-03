//! Settings & history window (eframe/egui), launched as
//! `whisper-catch settings` — from the tray menu or the shell.

use anyhow::Result;
use eframe::egui;

use crate::{autostart, config};
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

#[derive(PartialEq)]
enum Tab {
    History,
    Settings,
}

pub fn run() -> Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([520.0, 620.0])
            .with_min_inner_size([420.0, 400.0]),
        ..Default::default()
    };
    eframe::run_native(
        "whisper-catch",
        options,
        Box::new(|_cc| Ok(Box::new(App::new()) as Box<dyn eframe::App>)),
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
}

impl App {
    fn new() -> Self {
        let cfg = config::load().unwrap_or_default();
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
        }
    }

    fn reload_history(&mut self) {
        self.entries = history::load(500).unwrap_or_default();
        self.totals = history::totals();
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("tabs").show(ctx, |ui| {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.tab, Tab::History, "  History  ");
                ui.selectable_value(&mut self.tab, Tab::Settings, "  Settings  ");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let (n, w, s) = self.totals;
                    ui.label(
                        egui::RichText::new(format!(
                            "{w} words · {n} utterances · {:.0} min spoken",
                            s / 60.0
                        ))
                        .weak(),
                    );
                });
            });
            ui.add_space(4.0);
        });

        egui::CentralPanel::default().show(ctx, |ui| match self.tab {
            Tab::History => self.history_tab(ui),
            Tab::Settings => self.settings_tab(ui),
        });
    }
}

impl App {
    fn history_tab(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui.button("Reload").clicked() {
                self.reload_history();
            }
            if self.confirm_clear {
                ui.label("Really delete all history?");
                if ui.button("Yes, clear").clicked() {
                    let _ = history::clear();
                    self.reload_history();
                    self.confirm_clear = false;
                }
                if ui.button("Cancel").clicked() {
                    self.confirm_clear = false;
                }
            } else if ui.button("Clear…").clicked() {
                self.confirm_clear = true;
            }
        });
        ui.separator();

        if self.entries.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.label(egui::RichText::new("No transcriptions yet — hold the hotkey and speak.").weak());
            });
            return;
        }

        let mut copy: Option<String> = None;
        egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
            for e in &self.entries {
                ui.group(|ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        let when = chrono::DateTime::from_timestamp(e.ts as i64, 0)
                            .map(|t| {
                                t.with_timezone(&chrono::Local)
                                    .format("%b %d, %H:%M")
                                    .to_string()
                            })
                            .unwrap_or_default();
                        ui.label(egui::RichText::new(when).weak().small());
                        ui.label(
                            egui::RichText::new(format!("{:.1}s → {:.2}s", e.dur_s, e.infer_s))
                                .weak()
                                .small(),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.small_button("Copy").clicked() {
                                copy = Some(e.text.clone());
                            }
                        });
                    });
                    ui.label(&e.text);
                });
                ui.add_space(2.0);
            }
        });
        if let Some(text) = copy {
            ui.ctx().copy_text(text);
            self.status = "copied".into();
        }
    }

    fn settings_tab(&mut self, ui: &mut egui::Ui) {
        ui.add_space(8.0);
        egui::Grid::new("settings")
            .num_columns(2)
            .spacing([16.0, 12.0])
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

                ui.label("Start on login");
                ui.checkbox(&mut self.autostart_on, "");
                ui.end_row();

                ui.label("Keep history");
                ui.checkbox(&mut self.cfg.history, "");
                ui.end_row();

                ui.label("Model");
                ui.label(
                    self.cfg
                        .model_dir
                        .clone()
                        .unwrap_or_else(|| wc_core::models_dir().join("parakeet-tdt-0.6b-v2-int8"))
                        .display()
                        .to_string(),
                );
                ui.end_row();
            });

        ui.add_space(12.0);
        if ui.button("Save").clicked() {
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
                self.status = "saved — restart the daemon to apply key changes".into();
            }
        }
        if !self.status.is_empty() {
            ui.add_space(6.0);
            ui.label(egui::RichText::new(&self.status).weak());
        }
    }
}
