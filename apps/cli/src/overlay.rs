//! Floating recording indicator — a small always-on-top pill at the bottom
//! center of the screen while dictating. Runs as its own process
//! (`whisper-catch overlay`): the daemon spawns it on key-down, writes a
//! line to its stdin when transcription starts, and closes stdin (EOF) to
//! dismiss it.
//!
//! Look per docs/DESIGN.md: dark translucent rounded-full pill with a subtle
//! ring. Listening = red pulsing LED + 4-bar waveform + label + elapsed mono
//! timer behind a hairline. Transcribing = amber spinner + label + 3-dot
//! progress.

use std::io::BufRead;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;
use std::time::Instant;

use eframe::egui;

use crate::theme;

const W: f32 = 232.0;
const H: f32 = 40.0;

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
            theme::install_fonts(&cc.egui_ctx);
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
                started: Instant::now(),
                position_frames: 0,
                shot: crate::shot::Shot::from_env(),
            }) as Box<dyn eframe::App>)
        }),
    )
    .map_err(|e| anyhow::anyhow!("overlay failed: {e}"))
}

struct Overlay {
    state: Arc<AtomicU8>,
    /// Recording start ≈ overlay spawn (daemon spawns us on key-down).
    started: Instant,
    /// Re-assert the position for the first frames — some WMs override the
    /// first move request, leaving the pill wherever it was initially placed.
    position_frames: u32,
    shot: crate::shot::Shot,
}

impl eframe::App for Overlay {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0] // fully transparent backdrop
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.shot.tick(ctx);
        if self.state.load(Ordering::Relaxed) == STATE_DONE {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }

        if self.position_frames < 10 {
            if let Some(size) = ctx.input(|i| i.viewport().monitor_size) {
                let x = (size.x - W) / 2.0;
                let y = size.y - H - 64.0;
                ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition([x, y].into()));
                if self.position_frames == 0 {
                    log::info!("overlay at ({x:.0},{y:.0}) on monitor {size:?}");
                }
                self.position_frames += 1;
            } else if self.position_frames == 0 {
                log::warn!("overlay: monitor size unknown, using WM placement");
                self.position_frames = 10;
            }
        }

        let transcribing = self.state.load(Ordering::Relaxed) == STATE_TRANSCRIBING;
        let t = ctx.input(|i| i.time);

        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ctx, |ui| {
                let rect = ui.max_rect();
                let painter = ui.painter();
                // pill: zinc-950 at ~92%, subtle white ring
                painter.rect_filled(
                    rect,
                    H / 2.0,
                    egui::Color32::from_rgba_unmultiplied(9, 9, 11, 235),
                );
                painter.rect_stroke(
                    rect.shrink(0.5),
                    H / 2.0,
                    egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(255, 255, 255, 45)),
                    egui::StrokeKind::Inside,
                );

                let cy = rect.center().y;
                // right block starts after a hairline separator
                let sep_x = rect.right() - 62.0;
                painter.vline(
                    sep_x,
                    egui::Rangef::new(cy - 9.0, cy + 9.0),
                    egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(255, 255, 255, 26)),
                );

                if transcribing {
                    // amber spinner (1s linear) + label + 3-dot progress
                    let spin = egui::pos2(rect.left() + 22.0, cy);
                    let a0 = (t % 1.0) as f32 * std::f32::consts::TAU;
                    let pts: Vec<egui::Pos2> = (0..=20)
                        .map(|i| {
                            let a = a0 + i as f32 / 20.0 * std::f32::consts::TAU * 0.72;
                            spin + egui::vec2(a.cos() * 6.5, a.sin() * 6.5)
                        })
                        .collect();
                    painter.add(egui::Shape::line(
                        pts,
                        egui::Stroke::new(2.0, theme::AMBER),
                    ));
                    painter.text(
                        egui::pos2(rect.left() + 40.0, cy),
                        egui::Align2::LEFT_CENTER,
                        "Transcribing…",
                        theme::medium(13.0),
                        theme::FG,
                    );
                    // 3 dots, sequential pulse
                    for k in 0..3 {
                        let phase = (t * 2.0 - k as f64 * 0.25).fract();
                        let on = (1.0 - phase as f32).clamp(0.25, 1.0);
                        painter.circle_filled(
                            egui::pos2(sep_x + 19.0 + k as f32 * 12.0, cy),
                            2.5,
                            theme::AMBER.linear_multiply(on),
                        );
                    }
                } else {
                    // red LED, 2s ease pulse + soft halo
                    let pulse = 0.4 + 0.6 * (0.5 + 0.5 * (t * std::f64::consts::TAU / 2.0).cos() as f32);
                    let led = egui::pos2(rect.left() + 22.0, cy);
                    painter.circle_filled(led, 8.0, theme::RED.linear_multiply(0.18 * pulse));
                    painter.circle_filled(led, 4.0, theme::RED.linear_multiply(0.55 + 0.45 * pulse));

                    // 4-bar waveform
                    for k in 0..4 {
                        let phase = t * 6.3 + k as f64 * 1.7;
                        let h = 4.0 + 10.0 * phase.sin().abs() as f32;
                        let x = rect.left() + 40.0 + k as f32 * 7.0;
                        painter.rect_filled(
                            egui::Rect::from_center_size(egui::pos2(x, cy), egui::vec2(3.0, h)),
                            1.5,
                            theme::RED,
                        );
                    }
                    painter.text(
                        egui::pos2(rect.left() + 72.0, cy),
                        egui::Align2::LEFT_CENTER,
                        "Listening…",
                        theme::medium(13.0),
                        theme::FG,
                    );
                    // elapsed timer, mono, right-aligned
                    let secs = self.started.elapsed().as_secs();
                    painter.text(
                        egui::pos2(rect.right() - 16.0, cy),
                        egui::Align2::RIGHT_CENTER,
                        format!("{}:{:02}", secs / 60, secs % 60),
                        egui::FontId::monospace(12.0),
                        egui::Color32::from_rgb(161, 161, 170),
                    );
                }
            });

        ctx.request_repaint_after(std::time::Duration::from_millis(33));
    }
}
