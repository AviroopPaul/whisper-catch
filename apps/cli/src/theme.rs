//! Shared look & feel for all app windows.

use eframe::egui::{self, Color32};

/// Brand accent — matches the landing page teal.
pub const ACCENT_DARK: Color32 = Color32::from_rgb(94, 234, 212);
pub const ACCENT_LIGHT: Color32 = Color32::from_rgb(13, 148, 136);

pub fn accent(ui: &egui::Ui) -> Color32 {
    if ui.visuals().dark_mode {
        ACCENT_DARK
    } else {
        ACCENT_LIGHT
    }
}

/// Applies theme preference + a styling pass over egui defaults:
/// roomier spacing, rounder corners, slightly larger base text.
pub fn apply(ctx: &egui::Context, pref: &str) {
    let tp = match pref {
        "light" => egui::ThemePreference::Light,
        "dark" => egui::ThemePreference::Dark,
        _ => egui::ThemePreference::System,
    };
    ctx.options_mut(|o| o.theme_preference = tp);

    ctx.all_styles_mut(|style| {
        style.spacing.item_spacing = egui::vec2(10.0, 8.0);
        style.spacing.button_padding = egui::vec2(14.0, 7.0);
        style.spacing.interact_size.y = 30.0;
        style.visuals.widgets.noninteractive.corner_radius = egui::CornerRadius::same(8);
        style.visuals.widgets.inactive.corner_radius = egui::CornerRadius::same(8);
        style.visuals.widgets.hovered.corner_radius = egui::CornerRadius::same(8);
        style.visuals.widgets.active.corner_radius = egui::CornerRadius::same(8);
        style.visuals.widgets.open.corner_radius = egui::CornerRadius::same(8);
        for (ts, font) in style.text_styles.iter_mut() {
            match ts {
                egui::TextStyle::Heading => font.size = 22.0,
                egui::TextStyle::Body | egui::TextStyle::Button => font.size = 15.0,
                egui::TextStyle::Small => font.size = 12.5,
                _ => {}
            }
        }
    });
}

/// A subtle card container for list entries and sections.
pub fn card(ui: &egui::Ui) -> egui::Frame {
    egui::Frame::group(ui.style())
        .fill(ui.visuals().faint_bg_color)
        .inner_margin(12.0)
}
