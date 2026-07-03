//! First-run setup wizard: Welcome → keyboard permission (via polkit, no
//! terminal) → model download with progress → Done. Runs before the daemon
//! starts. Fixed-size centered window, one accent action per screen; the
//! window itself is the card — whitespace does the work.

use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};

use anyhow::Result;
use eframe::egui;
use wc_models::ModelId;

use crate::theme;

pub enum Outcome {
    /// Setup finished; start the daemon.
    Ready,
    /// User closed the window before finishing.
    Cancelled,
}

enum Step {
    Welcome,
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

pub fn need_setup(model: ModelId) -> bool {
    !wc_hotkey::keyboard_accessible() || !model.spec().is_complete(&wc_core::models_dir())
}

/// Blocks on the wizard window; returns when setup is complete or abandoned.
pub fn run(theme_pref: &str, model: ModelId) -> Result<Outcome> {
    let outcome = Arc::new(Mutex::new(Outcome::Cancelled));
    let out = outcome.clone();
    let pref = theme_pref.to_string();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([520.0, 560.0])
            .with_min_inner_size([520.0, 560.0])
            .with_resizable(false),
        centered: true,
        ..Default::default()
    };
    eframe::run_native(
        "WhisprCatch Setup",
        options,
        Box::new(move |cc| {
            theme::apply(&cc.egui_ctx, &pref);
            Ok(Box::new(Wizard::new(out, model)) as Box<dyn eframe::App>)
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
    model: ModelId,
}

impl Wizard {
    fn new(outcome: Arc<Mutex<Outcome>>, model: ModelId) -> Self {
        Self {
            step: Step::Welcome,
            outcome,
            model,
        }
    }
}

impl Step {
    fn first_download(model: ModelId) -> Step {
        if model.spec().is_complete(&wc_core::models_dir()) {
            Step::Done
        } else {
            Step::Download {
                started: false,
                rx: None,
                file: String::new(),
                done: 0,
                total: model.spec().total_size(),
                error: None,
            }
        }
    }

    fn index(&self) -> usize {
        match self {
            Step::Welcome => 0,
            Step::Permission { .. } => 1,
            Step::Download { .. } => 2,
            Step::Done => 3,
        }
    }
}

/// Linux: grants keyboard access via polkit (GUI password prompt):
/// input-group membership for permanence + ACLs so it works immediately.
#[cfg(target_os = "linux")]
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

/// macOS: register the app for Accessibility (shows the system prompt) and open
/// the Input Monitoring + Microphone privacy panes. The user toggles WhisprCatch
/// on in each; grants may need an app relaunch to take effect.
#[cfg(target_os = "macos")]
fn grant_keyboard_access() -> Result<(), String> {
    let trusted = wc_hotkey::request_accessibility();
    for pane in [
        "com.apple.preference.security?Privacy_Accessibility",
        "com.apple.preference.security?Privacy_ListenEvent",
        "com.apple.preference.security?Privacy_Microphone",
    ] {
        let _ = std::process::Command::new("open")
            .arg(format!("x-apple.systempreferences:{pane}"))
            .status();
    }
    if trusted {
        Ok(())
    } else {
        Err("Turn on WhisprCatch under Accessibility, Input Monitoring, and \
             Microphone in the panels that opened, then reopen WhisprCatch."
            .into())
    }
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn grant_keyboard_access() -> Result<(), String> {
    Err("keyboard access setup is not implemented on this platform".into())
}

// ---------------------------------------------------------------------------
// Painted UI pieces (no icon fonts needed; crisp at any DPI, light + dark).
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
enum StepIcon {
    Mic,
    Keyboard,
    Download,
    Check,
}

/// Accent at ~10% alpha — chip/plate tint per the design brief.
fn accent_subtle(ui: &egui::Ui) -> egui::Color32 {
    let a = theme::accent(ui);
    let alpha = if ui.visuals().dark_mode { 26 } else { 20 };
    egui::Color32::from_rgba_unmultiplied(a.r(), a.g(), a.b(), alpha)
}

/// 48px stroke icon in accent, centered on a 72px accent-subtle circle plate.
fn icon_plate(ui: &mut egui::Ui, icon: StepIcon) {
    let accent = theme::accent(ui);
    let plate = accent_subtle(ui);
    let (rect, _) = ui.allocate_exact_size(egui::vec2(72.0, 72.0), egui::Sense::hover());
    let c = rect.center();
    let p = ui.painter();
    p.circle_filled(c, 36.0, plate);
    let s = egui::Stroke::new(2.5, accent);
    match icon {
        StepIcon::Mic => {
            // capsule body
            p.rect_stroke(
                egui::Rect::from_center_size(egui::pos2(c.x, c.y - 6.0), egui::vec2(13.0, 22.0)),
                egui::CornerRadius::same(6),
                s,
                egui::StrokeKind::Middle,
            );
            // holder arc
            let arc: Vec<egui::Pos2> = (0..=24)
                .map(|i| {
                    let t = std::f32::consts::PI * i as f32 / 24.0;
                    egui::pos2(c.x + 12.5 * t.cos(), (c.y - 2.0) + 12.5 * t.sin())
                })
                .collect();
            p.add(egui::Shape::line(arc, s));
            // stem + base
            p.line_segment([egui::pos2(c.x, c.y + 10.5), egui::pos2(c.x, c.y + 16.0)], s);
            p.line_segment(
                [egui::pos2(c.x - 7.0, c.y + 16.0), egui::pos2(c.x + 7.0, c.y + 16.0)],
                s,
            );
        }
        StepIcon::Keyboard => {
            p.rect_stroke(
                egui::Rect::from_center_size(c, egui::vec2(38.0, 26.0)),
                egui::CornerRadius::same(5),
                egui::Stroke::new(2.0, accent),
                egui::StrokeKind::Middle,
            );
            for y in [-6.0_f32, 0.0] {
                for k in -2..=2_i32 {
                    p.circle_filled(egui::pos2(c.x + k as f32 * 6.0, c.y + y), 1.5, accent);
                }
            }
            // space bar
            p.line_segment(
                [egui::pos2(c.x - 8.0, c.y + 6.5), egui::pos2(c.x + 8.0, c.y + 6.5)],
                egui::Stroke::new(2.5, accent),
            );
        }
        StepIcon::Download => {
            // arrow
            p.line_segment([egui::pos2(c.x, c.y - 15.0), egui::pos2(c.x, c.y + 3.0)], s);
            p.add(egui::Shape::line(
                vec![
                    egui::pos2(c.x - 7.0, c.y - 4.0),
                    egui::pos2(c.x, c.y + 3.5),
                    egui::pos2(c.x + 7.0, c.y - 4.0),
                ],
                s,
            ));
            // tray
            p.add(egui::Shape::line(
                vec![
                    egui::pos2(c.x - 13.0, c.y + 8.0),
                    egui::pos2(c.x - 13.0, c.y + 14.0),
                    egui::pos2(c.x + 13.0, c.y + 14.0),
                    egui::pos2(c.x + 13.0, c.y + 8.0),
                ],
                s,
            ));
        }
        StepIcon::Check => {
            p.circle_stroke(c, 16.0, s);
            p.add(egui::Shape::line(
                vec![
                    egui::pos2(c.x - 7.5, c.y + 0.5),
                    egui::pos2(c.x - 2.5, c.y + 6.0),
                    egui::pos2(c.x + 8.0, c.y - 5.5),
                ],
                egui::Stroke::new(3.0, accent),
            ));
        }
    }
}

/// Four 8px dots: done = accent fill, current = accent ring, upcoming = border.
fn step_dots(ui: &mut egui::Ui, current: usize) {
    let accent = theme::accent(ui);
    let upcoming = ui.visuals().widgets.noninteractive.bg_stroke.color;
    let n = 4;
    let r = 4.0;
    let gap = 12.0;
    let w = n as f32 * 2.0 * r + (n - 1) as f32 * gap;
    let (rect, _) = ui.allocate_exact_size(egui::vec2(w, 2.0 * r + 4.0), egui::Sense::hover());
    let p = ui.painter();
    for i in 0..n {
        let c = egui::pos2(
            rect.left() + r + i as f32 * (2.0 * r + gap),
            rect.center().y,
        );
        if i < current {
            p.circle_filled(c, r, accent);
        } else if i == current {
            p.circle_stroke(c, r + 0.5, egui::Stroke::new(1.5, accent));
        } else {
            p.circle_filled(c, r, upcoming);
        }
    }
}

/// Centered column for step copy — max 380px so lines stay readable.
fn step_body(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui)) {
    let w = ui.available_width().min(380.0);
    ui.allocate_ui_with_layout(
        egui::vec2(w, 0.0),
        egui::Layout::top_down(egui::Align::Center),
        add,
    );
}

/// The one accent-filled button per screen.
fn primary_button(ui: &mut egui::Ui, text: &str, min: egui::Vec2) -> egui::Response {
    let accent = theme::accent(ui);
    let on_accent = if ui.visuals().dark_mode {
        egui::Color32::from_rgb(4, 47, 44)
    } else {
        egui::Color32::WHITE
    };
    ui.add(
        egui::Button::new(egui::RichText::new(text).strong().color(on_accent))
            .fill(accent)
            .stroke(egui::Stroke::NONE)
            .min_size(min),
    )
}

/// Small accent-subtle pill with accent text (welcome privacy line).
fn accent_chip(ui: &mut egui::Ui, text: &str) {
    let accent = theme::accent(ui);
    let bg = accent_subtle(ui);
    egui::Frame::default()
        .fill(bg)
        .corner_radius(egui::CornerRadius::same(14))
        .inner_margin(egui::Margin::symmetric(12, 6))
        .show(ui, |ui| {
            ui.label(egui::RichText::new(text).color(accent).small().strong());
        });
}

/// Error state: tinted panel, error text, optional recovery hint.
fn error_box(ui: &mut egui::Ui, msg: &str, hint: Option<&str>) {
    let err = ui.visuals().error_fg_color;
    let bg = egui::Color32::from_rgba_unmultiplied(err.r(), err.g(), err.b(), 18);
    egui::Frame::default()
        .fill(bg)
        .corner_radius(egui::CornerRadius::same(8))
        .inner_margin(12.0)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.colored_label(err, msg);
            if let Some(h) = hint {
                ui.add_space(4.0);
                ui.label(egui::RichText::new(h).weak().small());
            }
        });
}

/// "Hold ⟨Right Alt⟩ and speak. Release to type." with a drawn kbd chip.
fn hotkey_line(ui: &mut egui::Ui) {
    let text_color = ui.visuals().text_color();
    let strong = ui.visuals().strong_text_color();
    let chip_fill = ui.visuals().faint_bg_color;
    let border = ui.visuals().widgets.noninteractive.bg_stroke.color;
    let body = egui::TextStyle::Body.resolve(ui.style());

    let pre = ui
        .fonts(|f| f.layout_no_wrap("Hold".into(), body.clone(), text_color));
    let post = ui
        .fonts(|f| f.layout_no_wrap("and speak. Release to type.".into(), body, text_color));
    let key = ui.fonts(|f| f.layout_no_wrap("Right Alt".into(), egui::FontId::monospace(13.0), strong));

    let pad = egui::vec2(10.0, 6.0);
    let chip_size = key.size() + pad * 2.0;
    let gap = 8.0;
    let total = egui::vec2(
        pre.size().x + gap + chip_size.x + gap + post.size().x,
        chip_size.y + 3.0,
    );
    let (rect, _) = ui.allocate_exact_size(total, egui::Sense::hover());
    let p = ui.painter();
    let cy = rect.center().y;
    let mut x = rect.left();

    p.galley(egui::pos2(x, cy - pre.size().y / 2.0), pre.clone(), text_color);
    x += pre.size().x + gap;

    let chip = egui::Rect::from_min_size(
        egui::pos2(x, cy - chip_size.y / 2.0 - 1.0),
        chip_size,
    );
    p.rect_filled(chip, egui::CornerRadius::same(6), chip_fill);
    p.rect_stroke(
        chip,
        egui::CornerRadius::same(6),
        egui::Stroke::new(1.0, border),
        egui::StrokeKind::Inside,
    );
    // the "key" bottom edge
    p.line_segment(
        [
            egui::pos2(chip.left() + 5.0, chip.bottom() + 1.5),
            egui::pos2(chip.right() - 5.0, chip.bottom() + 1.5),
        ],
        egui::Stroke::new(2.0, border),
    );
    p.galley(chip.min + pad, key, strong);
    x += chip_size.x + gap;

    p.galley(egui::pos2(x, cy - post.size().y / 2.0), post, text_color);
}

// ---------------------------------------------------------------------------

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
            _ => {}
        }
        if advance_to_download {
            self.step = Step::first_download(self.model);
        }

        let step_idx = self.step.index();
        let panel_fill = ctx.style().visuals.panel_fill;
        let mut next: Option<Step> = None;
        let model = self.model;

        // pinned action area — buttons live at a stable position near the bottom
        egui::TopBottomPanel::bottom("wizard-actions")
            .show_separator_line(false)
            .frame(
                egui::Frame::default()
                    .fill(panel_fill)
                    .inner_margin(egui::Margin { left: 24, right: 24, top: 8, bottom: 40 }),
            )
            .show(ctx, |ui| {
                ui.set_min_height(48.0);
                ui.vertical_centered(|ui| match &mut self.step {
                    Step::Welcome => {
                        if primary_button(ui, "Get started", egui::vec2(220.0, 40.0)).clicked() {
                            next = Some(if wc_hotkey::keyboard_accessible() {
                                Step::first_download(model)
                            } else {
                                Step::Permission {
                                    granting: false,
                                    rx: None,
                                    error: None,
                                }
                            });
                        }
                    }
                    Step::Permission { granting, rx, error } => {
                        if *granting {
                            ui.add(egui::Spinner::new().size(20.0).color(theme::accent(ui)));
                            ui.add_space(4.0);
                            ui.label(
                                egui::RichText::new("Waiting for authorization…").weak().small(),
                            );
                        } else if primary_button(
                            ui,
                            "Grant keyboard access…",
                            egui::vec2(240.0, 40.0),
                        )
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
                    }
                    Step::Download { .. } => {
                        ui.add_space(12.0);
                        ui.label(
                            egui::RichText::new(
                                "This happens once — later launches start straight away.",
                            )
                            .weak()
                            .small(),
                        );
                    }
                    Step::Done => {
                        if primary_button(ui, "Start dictating", egui::vec2(220.0, 44.0)).clicked()
                        {
                            *self.outcome.lock().unwrap() = Outcome::Ready;
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    }
                });
            });

        egui::CentralPanel::default()
            .frame(
                egui::Frame::default()
                    .fill(panel_fill)
                    .inner_margin(egui::Margin::symmetric(24, 0)),
            )
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(32.0);
                    step_dots(ui, step_idx);
                    ui.add_space(10.0);
                    ui.label(
                        egui::RichText::new(format!("Step {} of 4", step_idx + 1))
                            .weak()
                            .small(),
                    );
                    ui.add_space(24.0);
                    let icon = match &self.step {
                        Step::Welcome => StepIcon::Mic,
                        Step::Permission { .. } => StepIcon::Keyboard,
                        Step::Download { .. } => StepIcon::Download,
                        Step::Done => StepIcon::Check,
                    };
                    icon_plate(ui, icon);
                    ui.add_space(24.0);

                    match &mut self.step {
                        Step::Welcome => {
                            ui.label(
                                egui::RichText::new("Welcome to WhisprCatch").size(26.0).strong(),
                            );
                            ui.add_space(12.0);
                            step_body(ui, |ui| {
                                ui.label(
                                    "Push-to-talk dictation for your desktop. Hold a key, \
                                     speak, and your words are typed wherever your cursor is.",
                                );
                            });
                            ui.add_space(16.0);
                            accent_chip(ui, "Everything stays on this device");
                            ui.add_space(16.0);
                            step_body(ui, |ui| {
                                ui.label(
                                    egui::RichText::new(
                                        "Two quick steps: keyboard access, then the speech \
                                         model. You won't see this window again.",
                                    )
                                    .weak()
                                    .small(),
                                );
                            });
                        }
                        Step::Permission { error, .. } => {
                            ui.label(egui::RichText::new("Keyboard access").size(26.0).strong());
                            ui.add_space(12.0);
                            let perm_body = if cfg!(target_os = "macos") {
                                "macOS needs your permission for WhisprCatch to see the \
                                 hotkey and type for you — Accessibility, Input Monitoring, \
                                 and Microphone. The button below opens those settings."
                            } else {
                                "To notice when you hold the dictation key, WhisprCatch \
                                 needs permission to read your keyboard. You'll be asked \
                                 for your password once."
                            };
                            step_body(ui, |ui| {
                                ui.label(perm_body);
                            });
                            if let Some(e) = error {
                                ui.add_space(16.0);
                                let msg = e.clone();
                                step_body(ui, |ui| {
                                    error_box(
                                        ui,
                                        &msg,
                                        Some("Nothing was changed — you can try again below."),
                                    );
                                });
                            }
                        }
                        Step::Download { started, rx, file, done, total, error } => {
                            if !*started {
                                *started = true;
                                let (tx, r) = mpsc::channel();
                                *rx = Some(r);
                                let ctx2 = ui.ctx().clone();
                                std::thread::spawn(move || {
                                    let res = model.spec().ensure_with(
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
                            ui.label(egui::RichText::new("Speech model").size(26.0).strong());
                            ui.add_space(12.0);
                            let dl_line = format!(
                                "Downloading {} — about {} MB, one time. Everything runs \
                                 locally; audio never leaves this machine.",
                                model.label(),
                                model.download_mb(),
                            );
                            step_body(ui, |ui| {
                                ui.label(dl_line);
                            });
                            ui.add_space(24.0);
                            let frac = if *total > 0 {
                                *done as f32 / *total as f32
                            } else {
                                0.0
                            };
                            let mb_line = format!(
                                "{:.0}%  ·  {:.0} / {:.0} MB — {}",
                                frac * 100.0,
                                *done as f64 / 1e6,
                                *total as f64 / 1e6,
                                if file.is_empty() { "preparing…" } else { file.as_str() }
                            );
                            let err = error.clone();
                            step_body(ui, |ui| {
                                ui.add(
                                    egui::ProgressBar::new(frac)
                                        .desired_height(10.0)
                                        .fill(theme::accent(ui))
                                        .animate(err.is_none()),
                                );
                                ui.add_space(8.0);
                                ui.label(egui::RichText::new(mb_line).weak().small());
                                if let Some(e) = err {
                                    ui.add_space(16.0);
                                    error_box(
                                        ui,
                                        &e,
                                        Some(
                                            "Downloads resume where they left off — close and \
                                             reopen the app to retry.",
                                        ),
                                    );
                                }
                            });
                        }
                        Step::Done => {
                            ui.label(egui::RichText::new("You're all set.").size(26.0).strong());
                            ui.add_space(16.0);
                            hotkey_line(ui);
                            ui.add_space(20.0);
                            let accent = theme::accent(ui);
                            ui.allocate_ui_with_layout(
                                egui::vec2(300.0, 0.0),
                                egui::Layout::top_down(egui::Align::Min),
                                |ui| {
                                    ui.spacing_mut().item_spacing.y = 8.0;
                                    for line in [
                                        "Text lands wherever your cursor is.",
                                        "A small pill shows while it listens.",
                                        "History and settings live in the tray.",
                                    ] {
                                        ui.horizontal(|ui| {
                                            ui.label(
                                                egui::RichText::new("•").color(accent).strong(),
                                            );
                                            ui.label(line);
                                        });
                                    }
                                },
                            );
                            ui.add_space(20.0);
                            ui.label(
                                egui::RichText::new(
                                    "Built for people who think faster than they type.",
                                )
                                .weak()
                                .small()
                                .italics(),
                            );
                        }
                    }
                });
            });

        if let Some(s) = next {
            self.step = s;
            ctx.request_repaint();
        }

        match &self.step {
            Step::Permission { .. } | Step::Download { .. } => {
                ctx.request_repaint_after(std::time::Duration::from_millis(250));
            }
            _ => {}
        }
    }
}

/// Small fatal-error window for desktop launches (no terminal to print to).
/// macOS surfaces failures via a notification instead (GUI must be main-thread).
#[cfg_attr(target_os = "macos", allow(dead_code))]
pub fn error_window(message: &str, theme_pref: &str) {
    let msg = message.to_string();
    let pref = theme_pref.to_string();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([460.0, 240.0])
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

#[cfg_attr(target_os = "macos", allow(dead_code))]
struct ErrorApp {
    msg: String,
}

impl eframe::App for ErrorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let panel_fill = ctx.style().visuals.panel_fill;
        egui::CentralPanel::default()
            .frame(
                egui::Frame::default()
                    .fill(panel_fill)
                    .inner_margin(egui::Margin::same(20)),
            )
            .show(ctx, |ui| {
                ui.label(egui::RichText::new("Something went wrong").size(17.0).strong());
                ui.add_space(8.0);
                egui::ScrollArea::vertical().max_height(120.0).show(ui, |ui| {
                    ui.label(&self.msg);
                });
                ui.add_space(12.0);
                if ui.button("Close").clicked() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
    }
}
