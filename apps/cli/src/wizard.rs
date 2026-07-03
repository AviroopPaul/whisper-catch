//! First-run setup wizard: keyboard permission (via polkit, no terminal)
//! and model download with a progress bar. Runs before the daemon starts.

use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};

use anyhow::Result;
use eframe::egui;

use crate::theme;

pub enum Outcome {
    /// Setup finished; start the daemon.
    Ready,
    /// User closed the window before finishing.
    Cancelled,
}

enum Step {
    Permission {
        granting: bool,
        rx: Option<Receiver<Result<(), String>>>,
        error: Option<String>,
    },
    Download {
        started: bool,
        rx: Option<Receiver<DlMsg>>,
        file: String,
        done: u64,
        total: u64,
        error: Option<String>,
    },
    Done,
}

enum DlMsg {
    Progress { file: String, done: u64, total: u64 },
    Finished,
    Failed(String),
}

pub fn need_setup() -> bool {
    !wc_hotkey::keyboard_accessible()
        || !wc_models::PARAKEET_V2_INT8.is_complete(&wc_core::models_dir())
}

/// Blocks on the wizard window; returns when setup is complete or abandoned.
pub fn run(theme_pref: &str) -> Result<Outcome> {
    let outcome = Arc::new(Mutex::new(Outcome::Cancelled));
    let out = outcome.clone();
    let pref = theme_pref.to_string();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([460.0, 360.0])
            .with_resizable(false),
        centered: true,
        ..Default::default()
    };
    eframe::run_native(
        "WhisprCatch Setup",
        options,
        Box::new(move |cc| {
            theme::apply(&cc.egui_ctx, &pref);
            Ok(Box::new(Wizard::new(out)) as Box<dyn eframe::App>)
        }),
    )
    .map_err(|e| anyhow::anyhow!("setup window failed: {e}"))?;

    Ok(Arc::try_unwrap(outcome)
        .map(|m| m.into_inner().unwrap())
        .unwrap_or(Outcome::Cancelled))
}

struct Wizard {
    step: Step,
    outcome: Arc<Mutex<Outcome>>,
}

impl Wizard {
    fn new(outcome: Arc<Mutex<Outcome>>) -> Self {
        let step = if !wc_hotkey::keyboard_accessible() {
            Step::Permission {
                granting: false,
                rx: None,
                error: None,
            }
        } else {
            Step::first_download()
        };
        Self { step, outcome }
    }
}

impl Step {
    fn first_download() -> Step {
        if wc_models::PARAKEET_V2_INT8.is_complete(&wc_core::models_dir()) {
            Step::Done
        } else {
            Step::Download {
                started: false,
                rx: None,
                file: String::new(),
                done: 0,
                total: wc_models::PARAKEET_V2_INT8.total_size(),
                error: None,
            }
        }
    }
}

/// Grants keyboard access via polkit (GUI password prompt):
/// input-group membership for permanence + ACLs so it works immediately.
fn grant_keyboard_access() -> Result<(), String> {
    let user = std::env::var("USER").map_err(|_| "cannot determine username".to_string())?;
    let script = r#"set -e
usermod -aG input "$1"
setfacl -m "u:$1:r" /dev/input/event* 2>/dev/null || true
if [ -e /dev/uinput ]; then setfacl -m "u:$1:rw" /dev/uinput 2>/dev/null || true; fi"#;
    let status = std::process::Command::new("pkexec")
        .args(["sh", "-c", script, "sh", &user])
        .status()
        .map_err(|e| format!("could not run pkexec: {e}"))?;
    if !status.success() {
        return Err("authorization was cancelled or failed".into());
    }
    if !wc_hotkey::keyboard_accessible() {
        return Err(
            "access granted, but not active yet — log out and back in, then reopen the app"
                .into(),
        );
    }
    Ok(())
}

impl eframe::App for Wizard {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // poll background work
        let mut advance_to_download = false;
        match &mut self.step {
            Step::Permission { granting, rx, error } => {
                if let Some(r) = rx {
                    if let Ok(res) = r.try_recv() {
                        *granting = false;
                        *rx = None;
                        match res {
                            Ok(()) => advance_to_download = true,
                            Err(e) => *error = Some(e),
                        }
                    }
                }
            }
            Step::Download { rx, file, done, total, error, .. } => {
                let mut finished = false;
                if let Some(r) = rx {
                    while let Ok(msg) = r.try_recv() {
                        match msg {
                            DlMsg::Progress { file: f, done: d, total: t } => {
                                *file = f;
                                *done = d;
                                *total = t;
                            }
                            DlMsg::Finished => finished = true,
                            DlMsg::Failed(e) => *error = Some(e),
                        }
                    }
                }
                if finished {
                    self.step = Step::Done;
                }
            }
            Step::Done => {}
        }
        if advance_to_download {
            self.step = Step::first_download();
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_space(10.0);
            ui.vertical_centered(|ui| {
                ui.heading("WhisprCatch");
                ui.label(egui::RichText::new("one-time setup").weak());
            });
            ui.add_space(14.0);

            match &mut self.step {
                Step::Permission { granting, rx, error } => {
                    theme::card(ui).show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.label(egui::RichText::new("Step 1 of 2 — keyboard access").strong());
                        ui.add_space(4.0);
                        ui.label(
                            "To notice when you hold the dictation key, the app needs \
                             permission to read your keyboard. You'll be asked for your \
                             password once.",
                        );
                        ui.add_space(10.0);
                        if *granting {
                            ui.horizontal(|ui| {
                                ui.spinner();
                                ui.label("waiting for authorization…");
                            });
                        } else if ui
                            .button(egui::RichText::new("Grant keyboard access…").strong())
                            .clicked()
                        {
                            let (tx, r) = mpsc::channel();
                            *rx = Some(r);
                            *granting = true;
                            *error = None;
                            let ctx2 = ui.ctx().clone();
                            std::thread::spawn(move || {
                                let _ = tx.send(grant_keyboard_access());
                                ctx2.request_repaint();
                            });
                        }
                        if let Some(e) = error {
                            ui.add_space(6.0);
                            ui.colored_label(ui.visuals().error_fg_color, e.as_str());
                        }
                    });
                    ctx.request_repaint_after(std::time::Duration::from_millis(250));
                }
                Step::Download { started, rx, file, done, total, error } => {
                    theme::card(ui).show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.label(egui::RichText::new("Step 2 of 2 — speech model").strong());
                        ui.add_space(4.0);
                        ui.label(
                            "Downloading the on-device speech model (~660 MB, one time). \
                             Everything runs locally — audio never leaves this machine.",
                        );
                        ui.add_space(10.0);
                        if !*started {
                            *started = true;
                            let (tx, r) = mpsc::channel();
                            *rx = Some(r);
                            let ctx2 = ui.ctx().clone();
                            std::thread::spawn(move || {
                                let res = wc_models::PARAKEET_V2_INT8.ensure_with(
                                    &wc_core::models_dir(),
                                    &|f, d, t| {
                                        let _ = tx.send(DlMsg::Progress {
                                            file: f.to_string(),
                                            done: d,
                                            total: t,
                                        });
                                        ctx2.request_repaint();
                                    },
                                );
                                let _ = tx.send(match res {
                                    Ok(_) => DlMsg::Finished,
                                    Err(e) => DlMsg::Failed(format!("{e:#}")),
                                });
                                ctx2.request_repaint();
                            });
                        }
                        let frac = if *total > 0 {
                            *done as f32 / *total as f32
                        } else {
                            0.0
                        };
                        ui.add(egui::ProgressBar::new(frac).show_percentage().animate(true));
                        ui.label(
                            egui::RichText::new(format!(
                                "{:.0} / {:.0} MB — {}",
                                *done as f64 / 1e6,
                                *total as f64 / 1e6,
                                if file.is_empty() { "starting…" } else { file.as_str() }
                            ))
                            .weak()
                            .small(),
                        );
                        if let Some(e) = error {
                            ui.add_space(6.0);
                            ui.colored_label(ui.visuals().error_fg_color, e.as_str());
                            ui.label(
                                egui::RichText::new(
                                    "Downloads resume where they left off — close and reopen \
                                     the app to retry.",
                                )
                                .weak()
                                .small(),
                            );
                        }
                    });
                    ctx.request_repaint_after(std::time::Duration::from_millis(250));
                }
                Step::Done => {
                    theme::card(ui).show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.label(egui::RichText::new("All set").strong());
                        ui.add_space(4.0);
                        ui.label(
                            "Hold Right Alt, speak, release — your words are typed into \
                             whatever has focus. Find stats, history and settings in the \
                             tray icon.",
                        );
                        ui.add_space(12.0);
                        if ui
                            .button(egui::RichText::new("Start dictating").strong())
                            .clicked()
                        {
                            *self.outcome.lock().unwrap() = Outcome::Ready;
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    });
                }
            }
        });
    }
}

/// Small fatal-error window for desktop launches (no terminal to print to).
pub fn error_window(message: &str, theme_pref: &str) {
    let msg = message.to_string();
    let pref = theme_pref.to_string();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([440.0, 220.0])
            .with_resizable(false),
        centered: true,
        ..Default::default()
    };
    let _ = eframe::run_native(
        "WhisprCatch — error",
        options,
        Box::new(move |cc| {
            theme::apply(&cc.egui_ctx, &pref);
            Ok(Box::new(ErrorApp { msg }) as Box<dyn eframe::App>)
        }),
    );
}

struct ErrorApp {
    msg: String,
}

impl eframe::App for ErrorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_space(8.0);
            ui.heading("Something went wrong");
            ui.add_space(8.0);
            ui.label(&self.msg);
            ui.add_space(12.0);
            if ui.button("Close").clicked() {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        });
    }
}
