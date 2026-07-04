//! Shared look & feel for all app windows — "tactile engineer dark".
//!
//! Every value comes from docs/DESIGN.md (app sections): dark-only zinc
//! neutrals, three signal colors (red = recording, amber = processing,
//! green = active), Geist / Geist Mono embedded in the binary, radii
//! 4/6/10/14. Screens never hand-pick colors; they use the tokens and
//! helpers below.

use eframe::egui::{self, Color32, FontFamily, FontId};

// ---------------------------------------------------------------- palette
// zinc neutrals (dark-only; the app ships without a light theme)

pub const BG: Color32 = Color32::from_rgb(9, 9, 11); // zinc-950 window
pub const SURFACE: Color32 = Color32::from_rgb(24, 24, 27); // zinc-900 cards
pub const SURFACE_2: Color32 = Color32::from_rgb(39, 39, 42); // zinc-800 raised
pub const SURFACE_3: Color32 = Color32::from_rgb(52, 52, 58); // hover/active
pub const FG: Color32 = Color32::from_rgb(232, 232, 235); // zinc-100/200
pub const TEXT_2: Color32 = Color32::from_rgb(161, 161, 170); // zinc-400
pub const MUTED: Color32 = Color32::from_rgb(113, 113, 122); // zinc-500
/// 1px hairline — white at 8% (on zinc-950 ≈ rgb 29).
pub const BORDER: Color32 = Color32::from_rgb(29, 29, 32);
/// Focus/selected ring — white at 20%.
pub const RING: Color32 = Color32::from_rgb(58, 58, 62);

// signal colors — status only, never decoration
pub const RED: Color32 = Color32::from_rgb(239, 68, 68); // recording
pub const AMBER: Color32 = Color32::from_rgb(245, 158, 11); // processing / hotkey
pub const GREEN: Color32 = Color32::from_rgb(16, 185, 129); // active / ok

/// `color` at ~12% alpha — chip fills behind signal-colored text.
pub fn tint(color: Color32) -> Color32 {
    Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 30)
}

// ------------------------------------------------------------------ fonts

/// Geist (sans) + Geist Mono, embedded; egui-phosphor appended for icons.
/// Families: `Proportional` → Geist, `Monospace` → Geist Mono, plus named
/// "GeistMedium" / "GeistSemiBold" / "GeistMonoMedium" for emphasis (egui's
/// `strong()` only recolors — weight needs a family switch).
pub fn install_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    let data: [(&str, &[u8]); 5] = [
        ("geist", include_bytes!("../assets/fonts/Geist-Regular.ttf")),
        ("geist-medium", include_bytes!("../assets/fonts/Geist-Medium.ttf")),
        (
            "geist-semibold",
            include_bytes!("../assets/fonts/Geist-SemiBold.ttf"),
        ),
        (
            "geist-mono",
            include_bytes!("../assets/fonts/GeistMono-Regular.ttf"),
        ),
        (
            "geist-mono-medium",
            include_bytes!("../assets/fonts/GeistMono-Medium.ttf"),
        ),
    ];
    for (name, bytes) in data {
        fonts.font_data.insert(
            name.to_owned(),
            std::sync::Arc::new(egui::FontData::from_static(bytes)),
        );
    }
    egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);

    fonts
        .families
        .get_mut(&FontFamily::Proportional)
        .unwrap()
        .insert(0, "geist".to_owned());
    fonts
        .families
        .get_mut(&FontFamily::Monospace)
        .unwrap()
        .insert(0, "geist-mono".to_owned());

    let prop = fonts.families[&FontFamily::Proportional].clone();
    let mono = fonts.families[&FontFamily::Monospace].clone();
    for (family, face, base) in [
        ("GeistMedium", "geist-medium", &prop),
        ("GeistSemiBold", "geist-semibold", &prop),
        ("GeistMonoMedium", "geist-mono-medium", &mono),
    ] {
        let mut chain = base.clone();
        chain.insert(0, face.to_owned());
        fonts
            .families
            .insert(FontFamily::Name(family.into()), chain);
    }
    ctx.set_fonts(fonts);
}

pub fn medium(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("GeistMedium".into()))
}

pub fn semibold(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("GeistSemiBold".into()))
}

pub fn mono_medium(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("GeistMonoMedium".into()))
}

// ------------------------------------------------------------------ style

/// Full design-token pass over egui defaults. Dark-only.
pub fn apply(ctx: &egui::Context) {
    ctx.options_mut(|o| o.theme_preference = egui::ThemePreference::Dark);
    ctx.style_mut_of(egui::Theme::Dark, |style| {
        style.spacing.item_spacing = egui::vec2(8.0, 8.0);
        style.spacing.button_padding = egui::vec2(12.0, 6.0);
        style.spacing.interact_size.y = 28.0;
        for (ts, font) in style.text_styles.iter_mut() {
            match ts {
                egui::TextStyle::Heading => font.size = 17.0,
                egui::TextStyle::Body | egui::TextStyle::Button => font.size = 14.0,
                egui::TextStyle::Small => font.size = 11.5,
                egui::TextStyle::Monospace => font.size = 12.0,
                _ => {}
            }
        }

        let v = &mut style.visuals;
        v.panel_fill = BG;
        v.window_fill = SURFACE;
        v.window_stroke = egui::Stroke::new(1.0, BORDER);
        v.window_corner_radius = egui::CornerRadius::same(14);
        v.menu_corner_radius = egui::CornerRadius::same(10);
        v.faint_bg_color = SURFACE;
        v.extreme_bg_color = Color32::from_rgb(17, 17, 20); // inputs

        let r = egui::CornerRadius::same(6);
        for w in [
            &mut v.widgets.noninteractive,
            &mut v.widgets.inactive,
            &mut v.widgets.hovered,
            &mut v.widgets.active,
            &mut v.widgets.open,
        ] {
            w.corner_radius = r;
        }
        v.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, BORDER);
        v.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, FG);
        v.widgets.inactive.bg_fill = SURFACE_2;
        v.widgets.inactive.weak_bg_fill = SURFACE_2;
        v.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, BORDER);
        v.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, TEXT_2);
        v.widgets.hovered.bg_fill = SURFACE_3;
        v.widgets.hovered.weak_bg_fill = SURFACE_3;
        v.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, RING);
        v.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, FG);
        v.widgets.active.bg_fill = SURFACE_3;
        v.widgets.active.weak_bg_fill = SURFACE_3;
        v.widgets.active.bg_stroke = egui::Stroke::new(1.0, RING);
        v.widgets.active.fg_stroke = egui::Stroke::new(1.0, FG);
        v.widgets.open.bg_fill = SURFACE_2;
        v.widgets.open.weak_bg_fill = SURFACE_2;
        v.widgets.open.bg_stroke = egui::Stroke::new(1.0, RING);
        v.widgets.open.fg_stroke = egui::Stroke::new(1.0, FG);

        v.selection.bg_fill = Color32::from_rgba_unmultiplied(16, 185, 129, 55);
        v.selection.stroke = egui::Stroke::new(1.0, GREEN);
        v.hyperlink_color = FG;
        v.error_fg_color = RED;
        v.warn_fg_color = AMBER;
        v.override_text_color = None;
    });
}

// ------------------------------------------------------------- components

/// Card container: surface fill, hairline ring, radius 10, 16px inset.
pub fn card(_ui: &egui::Ui) -> egui::Frame {
    egui::Frame::default()
        .fill(SURFACE)
        .stroke(egui::Stroke::new(1.0, BORDER))
        .corner_radius(egui::CornerRadius::same(10))
        .inner_margin(16.0)
}

/// Small mono uppercase section label ("ENGINE PARAMETERS").
pub fn section_label(ui: &mut egui::Ui, text: &str) {
    ui.label(
        egui::RichText::new(text.to_uppercase())
            .font(mono_medium(11.0))
            .color(MUTED),
    );
}

/// Mono uppercase micro-text (timestamps, readouts).
pub fn mono_upper(text: &str, size: f32, color: Color32) -> egui::RichText {
    egui::RichText::new(text.to_uppercase())
        .font(FontId::monospace(size))
        .color(color)
}

/// Hotkey chip: amber mono uppercase on an amber tint, radius 4.
pub fn key_chip(ui: &mut egui::Ui, label: &str) {
    egui::Frame::default()
        .fill(tint(AMBER))
        .corner_radius(egui::CornerRadius::same(4))
        .inner_margin(egui::Margin::symmetric(8, 4))
        .show(ui, |ui| {
            ui.label(
                egui::RichText::new(label.to_uppercase())
                    .font(mono_medium(11.0))
                    .color(AMBER),
            );
        });
}

/// Status LED: small filled dot with a soft halo. `pulse` animates opacity
/// on a 2s cycle (caller must keep repainting, e.g. the overlay).
pub fn led(ui: &mut egui::Ui, color: Color32, pulse: bool) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(10.0, 10.0), egui::Sense::hover());
    let a = if pulse {
        let t = ui.input(|i| i.time);
        let phase = (t * std::f64::consts::TAU / 2.0).cos() as f32; // 2s cycle
        0.4 + 0.6 * (0.5 + 0.5 * phase)
    } else {
        1.0
    };
    let c = color.linear_multiply(a);
    let p = ui.painter();
    p.circle_filled(rect.center(), 7.0, color.linear_multiply(0.16 * a));
    p.circle_filled(rect.center(), 3.5, c);
}

/// Hardware-style toggle switch — signal green when on.
pub fn toggle(ui: &mut egui::Ui, on: &mut bool) -> egui::Response {
    let size = egui::vec2(36.0, 20.0);
    let (rect, mut resp) = ui.allocate_exact_size(size, egui::Sense::click());
    if resp.clicked() {
        *on = !*on;
        resp.mark_changed();
    }
    let t = ui.ctx().animate_bool_responsive(resp.id, *on);
    let mix = |a: Color32, b: Color32| {
        let (a, b) = (egui::Rgba::from(a), egui::Rgba::from(b));
        Color32::from(a * (1.0 - t) + b * t)
    };
    let p = ui.painter();
    p.rect_filled(rect, 10.0, mix(SURFACE_3, GREEN));
    p.rect_stroke(
        rect,
        10.0,
        egui::Stroke::new(1.0, mix(RING, GREEN)),
        egui::StrokeKind::Inside,
    );
    let x = egui::lerp((rect.left() + 10.0)..=(rect.right() - 10.0), t);
    p.circle_filled(egui::pos2(x, rect.center().y), 7.0, FG);
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// The one high-emphasis button per screen: zinc-100 fill, zinc-950 text.
pub fn primary_button(ui: &mut egui::Ui, text: impl Into<String>) -> egui::Response {
    ui.add(
        egui::Button::new(
            egui::RichText::new(text.into())
                .font(medium(13.5))
                .color(BG),
        )
        .fill(FG)
        .stroke(egui::Stroke::NONE)
        .corner_radius(egui::CornerRadius::same(6)),
    )
}
