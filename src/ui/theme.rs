use eframe::egui;

#[derive(Clone, Copy)]
pub struct Palette {
    pub background: egui::Color32,
    pub sidebar: egui::Color32,
    pub rail: egui::Color32,
    pub surface: egui::Color32,
    pub text: egui::Color32,
    pub muted: egui::Color32,
    pub border: egui::Color32,
    pub blue: egui::Color32,
}
impl Default for Palette {
    fn default() -> Self {
        Self {
            background: egui::Color32::from_rgb(24, 24, 24),
            sidebar: egui::Color32::from_rgb(27, 27, 27),
            rail: egui::Color32::from_rgb(33, 33, 33),
            surface: egui::Color32::from_rgb(48, 48, 48),
            text: egui::Color32::from_rgb(235, 235, 235),
            muted: egui::Color32::from_rgb(145, 145, 145),
            border: egui::Color32::from_rgb(48, 48, 48),
            blue: egui::Color32::from_rgb(45, 105, 202),
        }
    }
}
thread_local! { static PALETTE: std::cell::Cell<Palette> = std::cell::Cell::new(Palette::default()); }
pub fn set_palette(palette: Palette) {
    PALETTE.set(palette);
}
pub fn background() -> egui::Color32 {
    PALETTE.get().background
}
pub fn sidebar() -> egui::Color32 {
    PALETTE.get().sidebar
}
pub fn rail() -> egui::Color32 {
    PALETTE.get().rail
}
pub fn surface() -> egui::Color32 {
    PALETTE.get().surface
}
pub fn text() -> egui::Color32 {
    PALETTE.get().text
}
pub fn muted() -> egui::Color32 {
    PALETTE.get().muted
}
pub fn border() -> egui::Color32 {
    PALETTE.get().border
}
pub fn blue() -> egui::Color32 {
    PALETTE.get().blue
}

pub fn semibold(size: f32) -> egui::FontId {
    egui::FontId::new(size, egui::FontFamily::Name("semibold".into()))
}

pub fn conversation_scroll() -> egui::style::ScrollStyle {
    egui::style::ScrollStyle {
        bar_width: 12.0,
        handle_min_length: 36.0,
        bar_inner_margin: 6.0,
        bar_outer_margin: 12.0,
        foreground_color: true,
        ..egui::style::ScrollStyle::solid()
    }
}

pub fn apply(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    let font_directory = std::env::var_os("SystemRoot")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| "C:/Windows".into())
        .join("Fonts");

    if let Ok(bytes) = std::fs::read(font_directory.join("segoeui.ttf")) {
        fonts
            .font_data
            .insert("Segoe UI".into(), egui::FontData::from_owned(bytes).into());
        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .insert(0, "Segoe UI".into());
    }

    let mut bold_fonts = fonts.families[&egui::FontFamily::Proportional].clone();
    if let Ok(bytes) = std::fs::read(font_directory.join("seguisb.ttf")) {
        fonts.font_data.insert(
            "Segoe UI Semibold".into(),
            egui::FontData::from_owned(bytes).into(),
        );
        bold_fonts.insert(0, "Segoe UI Semibold".into());
    }
    fonts
        .families
        .insert(egui::FontFamily::Name("semibold".into()), bold_fonts);
    ctx.set_fonts(fonts);

    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = background();
    visuals.window_fill = sidebar();
    visuals.extreme_bg_color = background();
    visuals.faint_bg_color = egui::Color32::from_rgb(35, 35, 35);
    visuals.override_text_color = Some(text());
    visuals.selection.bg_fill = egui::Color32::from_rgb(28, 65, 119);
    visuals.widgets.inactive.bg_fill = egui::Color32::TRANSPARENT;
    visuals.widgets.inactive.weak_bg_fill = egui::Color32::TRANSPARENT;
    visuals.widgets.inactive.bg_stroke = egui::Stroke::NONE;
    visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0_f32, muted());
    visuals.widgets.hovered.bg_fill = surface();
    visuals.widgets.hovered.weak_bg_fill = surface();
    visuals.widgets.hovered.bg_stroke = egui::Stroke::NONE;
    visuals.widgets.active.bg_fill = surface();
    visuals.widgets.active.bg_stroke = egui::Stroke::NONE;
    visuals.window_corner_radius = egui::CornerRadius::same(14);
    visuals.menu_corner_radius = egui::CornerRadius::same(10);
    ctx.set_visuals(visuals);

    ctx.style_mut(|style| {
        style
            .text_styles
            .insert(egui::TextStyle::Body, egui::FontId::proportional(14.0));
        style
            .text_styles
            .insert(egui::TextStyle::Button, egui::FontId::proportional(14.0));
        style
            .text_styles
            .insert(egui::TextStyle::Heading, semibold(19.0));
        style
            .text_styles
            .insert(egui::TextStyle::Small, egui::FontId::proportional(12.0));
        style
            .text_styles
            .insert(egui::TextStyle::Monospace, egui::FontId::monospace(13.0));
        style.spacing.button_padding = egui::vec2(9.0, 6.0);
        style.spacing.item_spacing = egui::vec2(8.0, 8.0);
        style.spacing.interact_size.y = 28.0;
        style.spacing.scroll.bar_width = 5.0;
        style.spacing.scroll.floating = true;
    });
}
