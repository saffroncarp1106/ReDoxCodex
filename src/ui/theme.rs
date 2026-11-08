use eframe::egui;

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Palette {
    pub background: egui::Color32,
    pub sidebar: egui::Color32,
    pub rail: egui::Color32,
    pub surface: egui::Color32,
    pub input_background: egui::Color32,
    pub text: egui::Color32,
    pub menu_text: egui::Color32,
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
            input_background: egui::Color32::from_rgb(48, 48, 48),
            text: egui::Color32::from_rgb(235, 235, 235),
            menu_text: egui::Color32::from_rgb(128, 128, 128),
            muted: egui::Color32::from_rgb(145, 145, 145),
            border: egui::Color32::from_rgb(48, 48, 48),
            blue: egui::Color32::from_rgb(45, 105, 202),
        }
    }
}

#[derive(Clone, Default, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct TextColors {
    pub input_background: Option<[u8; 3]>,
    pub menu: Option<[u8; 3]>,
    pub general: Option<[u8; 3]>,
}

impl TextColors {
    pub fn apply(&self, mut palette: Palette) -> Palette {
        if let Some([r, g, b]) = self.input_background {
            palette.input_background = egui::Color32::from_rgb(r, g, b);
        }
        if let Some([r, g, b]) = self.menu {
            palette.menu_text = egui::Color32::from_rgb(r, g, b);
        }
        if let Some([r, g, b]) = self.general {
            palette.text = egui::Color32::from_rgb(r, g, b);
        }
        palette
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
pub fn input_background() -> egui::Color32 {
    PALETTE.get().input_background
}
pub fn text() -> egui::Color32 {
    PALETTE.get().text
}
pub fn menu_text() -> egui::Color32 {
    PALETTE.get().menu_text
}

pub fn update_colors(ctx: &egui::Context, palette: Palette) {
    if PALETTE.get() == palette {
        return;
    }
    set_palette(palette);
    ctx.style_mut(|style| {
        style.visuals.override_text_color = Some(palette.text);
    });
    ctx.request_repaint();
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

pub(super) fn slider(ui: &mut egui::Ui, slider: egui::Slider<'_>) -> egui::Response {
    if ui.visuals().dark_mode {
        return ui.add(slider);
    }
    ui.scope(|ui| {
        ui.spacing_mut().slider_rail_height = 2.0;
        ui.visuals_mut().widgets.inactive.bg_fill = egui::Color32::from_gray(155);
        ui.add(slider)
    })
    .inner
}

pub fn apply(ctx: &egui::Context, layout: &str) {
    let classic = layout == super::win98::ID;
    let mut fonts = egui::FontDefinitions::default();
    let font_directory = std::env::var_os("SystemRoot")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| "C:/Windows".into())
        .join("Fonts");

    let (regular, bold) = if classic {
        ("tahoma.ttf", "tahomabd.ttf")
    } else {
        ("segoeui.ttf", "seguisb.ttf")
    };
    if let Ok(bytes) = std::fs::read(font_directory.join(regular)) {
        fonts
            .font_data
            .insert(regular.into(), egui::FontData::from_owned(bytes).into());
        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .insert(0, regular.into());
    }

    let mut bold_fonts = fonts.families[&egui::FontFamily::Proportional].clone();
    if let Ok(bytes) = std::fs::read(font_directory.join(bold)) {
        fonts
            .font_data
            .insert(bold.into(), egui::FontData::from_owned(bytes).into());
        bold_fonts.insert(0, bold.into());
    }
    fonts
        .families
        .insert(egui::FontFamily::Name("semibold".into()), bold_fonts);
    let mut serif_fonts = fonts.families[&egui::FontFamily::Proportional].clone();
    if let Ok(bytes) = std::fs::read(font_directory.join("georgia.ttf")) {
        fonts
            .font_data
            .insert("Georgia".into(), egui::FontData::from_owned(bytes).into());
        serif_fonts.insert(0, "Georgia".into());
    }
    fonts
        .families
        .insert(egui::FontFamily::Name("serif".into()), serif_fonts);
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
    if classic {
        super::win98::apply_style(ctx);
    }
}

#[cfg(test)]
#[path = "../../tests/ui/text_colors.rs"]
mod text_color_tests;
