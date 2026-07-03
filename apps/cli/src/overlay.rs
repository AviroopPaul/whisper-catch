//! Floating recording indicator — a small always-on-top pill at the bottom
//! center of the screen while dictating (Wispr Flow style). Runs as its own
//! process (`whisper-catch overlay`): the daemon spawns it on key-down,
//! writes a line to its stdin when transcription starts, and closes stdin
//! (EOF) to dismiss it.

use std::io::BufRead;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;

use eframe::egui;

const W: f32 = 172.0;
const H: f32 = 48.0;

const STATE_LISTENING: u8 = 0;
const STATE_TRANSCRIBING: u8 = 1;
const STATE_DONE: u8 = 2;

pub fn run() -> anyhow::Result<()> {
    let state = Arc::new(AtomicU8::new(STATE_LISTENING));

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([W, H])
            .with_decorations(false)
            .with_transparent(true)
            .with_window_level(egui::WindowLevel::AlwaysOnTop)
            .with_taskbar(false)
            .with_resizable(false)
            // never take focus: keystrokes must keep flowing to the app the
            // user is dictating into (focus steal = streamed text lost)
            .with_active(false)
            .with_mouse_passthrough(true)
            .with_window_type(egui::X11WindowType::Notification),
        ..Default::default()
    };
    let st = state.clone();
    eframe::run_native(
        "WhisprCatch",
        options,
        Box::new(move |cc| {
            // stdin watcher: any line -> transcribing, EOF -> done
            let ctx = cc.egui_ctx.clone();
            let s = st.clone();
            std::thread::spawn(move || {
                let stdin = std::io::stdin();
                for line in stdin.lock().lines() {
                    if line.is_err() {
                        break;
                    }
                    s.store(STATE_TRANSCRIBING, Ordering::Relaxed);
                    ctx.request_repaint();
                }
                s.store(STATE_DONE, Ordering::Relaxed);
                ctx.request_repaint();
            });
            Ok(Box::new(Overlay {
                state: st,
                positioned: false,
            }) as Box<dyn eframe::App>)
        }),
    )
    .map_err(|e| anyhow::anyhow!("overlay failed: {e}"))
}

struct Overlay {
    state: Arc<AtomicU8>,
    positioned: bool,
}

impl eframe::App for Overlay {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0] // fully transparent backdrop
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.state.load(Ordering::Relaxed) == STATE_DONE {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }

        if !self.positioned {
            if let Some(size) = ctx.input(|i| i.viewport().monitor_size) {
                let x = (size.x - W) / 2.0;
                let y = size.y - H - 64.0;
                ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition([x, y].into()));
                self.positioned = true;
            }
        }

        let transcribing = self.state.load(Ordering::Relaxed) == STATE_TRANSCRIBING;
        let t = ctx.input(|i| i.time);

        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ctx, |ui| {
                let rect = ui.max_rect();
                let painter = ui.painter();
                painter.rect_filled(
                    rect,
                    H / 2.0,
                    egui::Color32::from_rgba_unmultiplied(16, 20, 30, 235),
                );
                painter.rect_stroke(
                    rect.shrink(0.5),
                    H / 2.0,
                    egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(94, 234, 212, 60)),
                    egui::StrokeKind::Inside,
                );

                let center_y = rect.center().y;
                if transcribing {
                    // spinner + label
                    let spin = egui::pos2(rect.left() + 26.0, center_y);
                    for k in 0..8 {
                        let a = t as f32 * 6.0 + k as f32 * std::f32::consts::FRAC_PI_4;
                        let alpha = ((k as f32 / 8.0) * 200.0) as u8;
                        painter.circle_filled(
                            spin + egui::vec2(a.cos() * 8.0, a.sin() * 8.0),
                            2.2,
                            egui::Color32::from_rgba_unmultiplied(94, 234, 212, alpha),
                        );
                    }
                    painter.text(
                        egui::pos2(rect.left() + 46.0, center_y),
                        egui::Align2::LEFT_CENTER,
                        "Transcribing…",
                        egui::FontId::proportional(14.0),
                        egui::Color32::from_rgb(230, 233, 240),
                    );
                } else {
                    // pulsing red dot + animated level bars
                    let pulse = (0.6 + 0.4 * (t * 4.0).sin() as f32).clamp(0.0, 1.0);
                    painter.circle_filled(
                        egui::pos2(rect.left() + 26.0, center_y),
                        6.0 + pulse * 2.0,
                        egui::Color32::from_rgba_unmultiplied(239, 68, 68, 180 + (pulse * 75.0) as u8),
                    );
                    painter.text(
                        egui::pos2(rect.left() + 46.0, center_y),
                        egui::Align2::LEFT_CENTER,
                        "Listening…",
                        egui::FontId::proportional(14.0),
                        egui::Color32::from_rgb(230, 233, 240),
                    );
                    // little animated bars on the right
                    for k in 0..4 {
                        let phase = t * 7.0 + k as f64 * 1.1;
                        let h = 5.0 + 9.0 * (phase.sin().abs()) as f32;
                        let x = rect.right() - 44.0 + k as f32 * 8.0;
                        painter.rect_filled(
                            egui::Rect::from_center_size(
                                egui::pos2(x, center_y),
                                egui::vec2(4.0, h),
                            ),
                            2.0,
                            egui::Color32::from_rgb(94, 234, 212),
                        );
                    }
                }
            });

        ctx.request_repaint_after(std::time::Duration::from_millis(33));
    }
}
