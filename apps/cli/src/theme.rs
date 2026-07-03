//! Shared look & feel for all app windows.
//!
//! Every color below comes from docs/DESIGN.md §2 — set centrally here so
//! screens never hand-pick colors. Public API used by other windows:
//! `apply(ctx, pref)`, `accent(ui)`, `card(ui)` (stable signatures), plus
//! additive helpers (`install_icons`, `accent_button`, semantic colors).

use eframe::egui::{self, Color32};

// ---------------------------------------------------------------- palette

/// Brand accent — matches the landing page teal.
pub const ACCENT_DARK: Color32 = Color32::from_rgb(94, 234, 212); // teal-300
pub const ACCENT_LIGHT: Color32 = Color32::from_rgb(13, 148, 136); // teal-600

// Dark neutrals (cool slate)
const D_BG0: Color32 = Color32::from_rgb(11, 14, 20); // window
const D_BG1: Color32 = Color32::from_rgb(17, 21, 31); // cards / raised
const D_BG2: Color32 = Color32::from_rgb(21, 26, 38); // inputs / buttons
const D_BG_HOVER: Color32 = Color32::from_rgb(26, 32, 46);
const D_BG_ACTIVE: Color32 = Color32::from_rgb(31, 38, 54);
const D_BORDER: Color32 = Color32::from_rgb(35, 42, 58);
const D_BORDER_STRONG: Color32 = Color32::from_rgb(51, 65, 85);
const D_TEXT: Color32 = Color32::from_rgb(230, 233, 240);
const D_ON_ACCENT: Color32 = Color32::from_rgb(4, 47, 44);
const D_SUCCESS: Color32 = Color32::from_rgb(52, 211, 153);
const D_ERROR: Color32 = Color32::from_rgb(248, 113, 113);
const D_WARNING: Color32 = Color32::from_rgb(251, 191, 36);

// Light neutrals
const L_BG0: Color32 = Color32::from_rgb(248, 250, 252);
const L_CARD: Color32 = Color32::from_rgb(255, 255, 255);
const L_BG_HOVER: Color32 = Color32::from_rgb(241, 245, 249);
const L_BG_ACTIVE: Color32 = Color32::from_rgb(226, 232, 240);
const L_BORDER: Color32 = Color32::from_rgb(226, 232, 240);
const L_BORDER_STRONG: Color32 = Color32::from_rgb(203, 213, 225);
const L_TEXT: Color32 = Color32::from_rgb(15, 23, 42);
const L_ON_ACCENT: Color32 = Color32::from_rgb(255, 255, 255);
const L_SUCCESS: Color32 = Color32::from_rgb(5, 150, 105);
const L_ERROR: Color32 = Color32::from_rgb(220, 38, 38);
const L_WARNING: Color32 = Color32::from_rgb(217, 119, 6);

// ------------------------------------------------------- semantic helpers

pub fn accent(ui: &egui::Ui) -> Color32 {
    if ui.visuals().dark_mode {
        ACCENT_DARK
    } else {
        ACCENT_LIGHT
    }
}

/// ~10% alpha accent — chip fills, icon plates, selected states.
pub fn accent_subtle(ui: &egui::Ui) -> Color32 {
    if ui.visuals().dark_mode {
        Color32::from_rgba_unmultiplied(94, 234, 212, 26)
    } else {
        Color32::from_rgba_unmultiplied(13, 148, 136, 20)
    }
}

/// Text color on top of an accent-filled surface.
pub fn on_accent(ui: &egui::Ui) -> Color32 {
    if ui.visuals().dark_mode {
        D_ON_ACCENT
    } else {
        L_ON_ACCENT
    }
}

pub fn success(ui: &egui::Ui) -> Color32 {
    if ui.visuals().dark_mode {
        D_SUCCESS
    } else {
        L_SUCCESS
    }
}

/// Warning tone — use sparingly (DESIGN.md §2). Kept for other windows.
#[allow(dead_code)]
pub fn warning(ui: &egui::Ui) -> Color32 {
    if ui.visuals().dark_mode {
        D_WARNING
    } else {
        L_WARNING
    }
}

/// 1px hairline color (matches card strokes).
pub fn border(ui: &egui::Ui) -> Color32 {
    if ui.visuals().dark_mode {
        D_BORDER
    } else {
        L_BORDER
    }
}

/// Hovered card/control border.
pub fn border_strong(ui: &egui::Ui) -> Color32 {
    if ui.visuals().dark_mode {
        D_BORDER_STRONG
    } else {
        L_BORDER_STRONG
    }
}

// ------------------------------------------------------------------ fonts

/// Adds the Phosphor icon font on top of egui's defaults so any window can
/// render `egui_phosphor::regular::*` glyphs inline in text. Call once per
/// context, after `eframe` is up (safe to call again — it just resets fonts).
pub fn install_icons(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
    ctx.set_fonts(fonts);
}

// ------------------------------------------------------------------ style

/// Applies theme preference + the full design-token pass over egui defaults
/// for BOTH dark and light styles (docs/DESIGN.md §2 egui mapping).
pub fn apply(ctx: &egui::Context, pref: &str) {
    let tp = match pref {
        "light" => egui::ThemePreference::Light,
        "dark" => egui::ThemePreference::Dark,
        _ => egui::ThemePreference::System,
    };
    ctx.options_mut(|o| o.theme_preference = tp);

    ctx.style_mut_of(egui::Theme::Dark, |style| {
        base_style(style);
        style.visuals = dark_visuals();
    });
    ctx.style_mut_of(egui::Theme::Light, |style| {
        base_style(style);
        style.visuals = light_visuals();
    });
}

fn base_style(style: &mut egui::Style) {
    style.spacing.item_spacing = egui::vec2(10.0, 8.0);
    style.spacing.button_padding = egui::vec2(14.0, 7.0);
    style.spacing.interact_size.y = 30.0;
    for (ts, font) in style.text_styles.iter_mut() {
        match ts {
            egui::TextStyle::Heading => font.size = 22.0,
            egui::TextStyle::Body | egui::TextStyle::Button => font.size = 15.0,
            egui::TextStyle::Small => font.size = 12.5,
            egui::TextStyle::Monospace => font.size = 13.0,
            _ => {}
        }
    }
}

fn round_widgets(v: &mut egui::Visuals) {
    let r = egui::CornerRadius::same(8);
    v.widgets.noninteractive.corner_radius = r;
    v.widgets.inactive.corner_radius = r;
    v.widgets.hovered.corner_radius = r;
    v.widgets.active.corner_radius = r;
    v.widgets.open.corner_radius = r;
    v.window_corner_radius = egui::CornerRadius::same(12);
    v.menu_corner_radius = r;
}

fn dark_visuals() -> egui::Visuals {
    let mut v = egui::Visuals::dark();
    v.panel_fill = D_BG0;
    v.window_fill = D_BG1;
    v.window_stroke = egui::Stroke::new(1.0, D_BORDER);
    v.faint_bg_color = D_BG1; // card fill
    v.extreme_bg_color = D_BG2; // text inputs

    v.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, D_BORDER);
    v.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, D_TEXT);
    v.widgets.inactive.bg_fill = D_BG2;
    v.widgets.inactive.weak_bg_fill = D_BG2;
    v.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, D_BORDER);
    v.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, D_TEXT);
    v.widgets.hovered.bg_fill = D_BG_HOVER;
    v.widgets.hovered.weak_bg_fill = D_BG_HOVER;
    v.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, D_BORDER_STRONG);
    v.widgets.hovered.fg_stroke = egui::Stroke::new(1.5, D_TEXT);
    v.widgets.active.bg_fill = D_BG_ACTIVE;
    v.widgets.active.weak_bg_fill = D_BG_ACTIVE;
    v.widgets.active.bg_stroke = egui::Stroke::new(1.0, D_BORDER_STRONG);
    v.widgets.active.fg_stroke = egui::Stroke::new(1.5, D_TEXT);
    v.widgets.open.bg_fill = D_BG1;
    v.widgets.open.weak_bg_fill = D_BG1;
    v.widgets.open.bg_stroke = egui::Stroke::new(1.0, D_BORDER_STRONG);
    v.widgets.open.fg_stroke = egui::Stroke::new(1.0, D_TEXT);

    v.selection.bg_fill = Color32::from_rgba_unmultiplied(94, 234, 212, 64);
    v.selection.stroke = egui::Stroke::new(1.0, ACCENT_DARK);
    v.hyperlink_color = ACCENT_DARK;
    v.error_fg_color = D_ERROR;
    v.warn_fg_color = D_WARNING;
    round_widgets(&mut v);
    v
}

fn light_visuals() -> egui::Visuals {
    let mut v = egui::Visuals::light();
    v.panel_fill = L_BG0;
    v.window_fill = L_CARD;
    v.window_stroke = egui::Stroke::new(1.0, L_BORDER);
    v.faint_bg_color = L_CARD; // card fill (differentiated by border)
    v.extreme_bg_color = L_CARD; // text inputs

    v.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, L_BORDER);
    v.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, L_TEXT);
    v.widgets.inactive.bg_fill = L_CARD;
    v.widgets.inactive.weak_bg_fill = L_CARD;
    v.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, L_BORDER);
    v.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, L_TEXT);
    v.widgets.hovered.bg_fill = L_BG_HOVER;
    v.widgets.hovered.weak_bg_fill = L_BG_HOVER;
    v.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, L_BORDER_STRONG);
    v.widgets.hovered.fg_stroke = egui::Stroke::new(1.5, L_TEXT);
    v.widgets.active.bg_fill = L_BG_ACTIVE;
    v.widgets.active.weak_bg_fill = L_BG_ACTIVE;
    v.widgets.active.bg_stroke = egui::Stroke::new(1.0, L_BORDER_STRONG);
    v.widgets.active.fg_stroke = egui::Stroke::new(1.5, L_TEXT);
    v.widgets.open.bg_fill = L_CARD;
    v.widgets.open.weak_bg_fill = L_CARD;
    v.widgets.open.bg_stroke = egui::Stroke::new(1.0, L_BORDER_STRONG);
    v.widgets.open.fg_stroke = egui::Stroke::new(1.0, L_TEXT);

    v.selection.bg_fill = Color32::from_rgba_unmultiplied(13, 148, 136, 50);
    v.selection.stroke = egui::Stroke::new(1.0, ACCENT_LIGHT);
    v.hyperlink_color = ACCENT_LIGHT;
    v.error_fg_color = L_ERROR;
    v.warn_fg_color = L_WARNING;
    round_widgets(&mut v);
    v
}

// ------------------------------------------------------------- containers

/// A subtle card container for list entries and sections.
/// r-lg 12, 16px inner margin, bg-step fill + 1px hairline (DESIGN.md §3).
pub fn card(ui: &egui::Ui) -> egui::Frame {
    egui::Frame::group(ui.style())
        .fill(ui.visuals().faint_bg_color)
        .stroke(egui::Stroke::new(1.0, border(ui)))
        .corner_radius(egui::CornerRadius::same(12))
        .inner_margin(16.0)
}

/// The ONE accent-filled button allowed per screen (DESIGN.md §5).
pub fn accent_button(ui: &mut egui::Ui, text: impl Into<String>) -> egui::Response {
    let fill = accent(ui);
    let fg = on_accent(ui);
    ui.add(
        egui::Button::new(egui::RichText::new(text.into()).strong().color(fg))
            .fill(fill)
            .corner_radius(egui::CornerRadius::same(8)),
    )
}
