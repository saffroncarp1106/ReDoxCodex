use eframe::egui::{
    self, Align, Align2, Color32, FontId, Frame, Layout, Margin, Rect, Response, Sense, Stroke, Ui,
    vec2,
};

use super::{
    conversation::render_transcript,
    icons::{self, Icon},
    theme,
};
use crate::app::ReDox;

pub(super) const ID: &str = "wii";
pub(super) const BLUE: Color32 = Color32::from_rgb(52, 190, 237);
const SETTINGS_KEY: &str = "wii_settings";
const QUIET_KEY: &str = "wii_quiet";
const FEEDBACK_KEY: &str = "wii_feedback";
const SILVER: Color32 = Color32::from_rgb(227, 232, 239);
const PLAYERS: [Color32; 4] = [
    Color32::from_rgb(0, 140, 255),
    Color32::from_rgb(232, 67, 70),
    Color32::from_rgb(44, 173, 101),
    Color32::from_rgb(218, 159, 20),
];

#[derive(Clone, Copy, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub(in crate::app) struct Settings {
    pub sounds: bool,
    pub motion: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            sounds: true,
            motion: true,
        }
    }
}

fn settings(context: &egui::Context) -> Settings {
    context
        .data(|data| data.get_temp::<Settings>(egui::Id::new(SETTINGS_KEY)))
        .unwrap_or_default()
}

pub(super) fn popup_feedback(context: &egui::Context) {
    let quiet = context
        .data(|data| data.get_temp::<bool>(egui::Id::new(QUIET_KEY)))
        .unwrap_or(true);
    finish_frame(context, settings(context).sounds, None, false, quiet);
}

#[derive(Clone, Default)]
struct Feedback {
    hovered: Option<egui::Id>,
    ready: bool,
    run: Option<redox::domain::RunId>,
}

#[derive(Clone, Copy)]
enum Sound {
    Hover,
    Click,
    Open,
    Success,
}

fn play(sound: Sound) {
    #[cfg(windows)]
    {
        use windows_sys::Win32::Media::Audio::{
            PlaySoundW, SND_ASYNC, SND_MEMORY, SND_NODEFAULT, SND_NOSTOP,
        };

        let bytes: &'static [u8] = match sound {
            Sound::Hover => include_bytes!("assets/wii/sounds/Hover.wav"),
            Sound::Click => include_bytes!("assets/wii/sounds/Click.wav"),
            Sound::Open => include_bytes!("assets/wii/sounds/Bloomp.wav"),
            Sound::Success => include_bytes!("assets/wii/sounds/Succes.wav"),
        };
        let flags = SND_ASYNC
            | SND_MEMORY
            | SND_NODEFAULT
            | if matches!(sound, Sound::Hover) {
                SND_NOSTOP
            } else {
                0
            };

        // WinMM retains this pointer during asynchronous playback; embedded bytes live for the process.
        unsafe {
            PlaySoundW(bytes.as_ptr().cast(), std::ptr::null_mut(), flags);
        }
    }
    #[cfg(not(windows))]
    let _ = sound;
}

pub(super) fn apply_style(context: &egui::Context) {
    theme::set_controls(context, theme::Controls::Wii);
    let mut visuals = egui::Visuals::light();
    visuals.panel_fill = theme::background();
    visuals.window_fill = theme::sidebar();
    visuals.extreme_bg_color = theme::input_background();
    visuals.faint_bg_color = theme::surface();
    visuals.code_bg_color = Color32::from_rgb(237, 241, 244);
    visuals.override_text_color = Some(theme::text());
    visuals.hyperlink_color = Color32::from_rgb(0, 116, 177);
    visuals.selection.bg_fill = Color32::from_rgb(176, 228, 249);
    visuals.selection.stroke = Stroke::new(1.0, theme::text());
    visuals.window_corner_radius = egui::CornerRadius::same(18);
    visuals.menu_corner_radius = egui::CornerRadius::same(16);
    visuals.window_stroke = Stroke::new(1.5, BLUE);
    visuals.window_shadow = egui::Shadow {
        offset: [0, 5],
        blur: 16,
        spread: 0,
        color: Color32::from_black_alpha(30),
    };

    let widgets = &mut visuals.widgets;
    for widget in [
        &mut widgets.inactive,
        &mut widgets.hovered,
        &mut widgets.active,
        &mut widgets.open,
    ] {
        widget.corner_radius = egui::CornerRadius::same(18);
        widget.bg_fill = SILVER;
        widget.weak_bg_fill = Color32::from_rgb(242, 245, 248);
        widget.bg_stroke = Stroke::new(1.0, theme::border());
        widget.fg_stroke = Stroke::new(1.25, theme::text());
        widget.expansion = 0.0;
    }
    widgets.hovered.bg_stroke = Stroke::new(2.0, BLUE);
    widgets.hovered.weak_bg_fill = Color32::WHITE;
    widgets.active.bg_fill = Color32::from_rgb(192, 226, 242);
    widgets.active.weak_bg_fill = widgets.active.bg_fill;
    widgets.active.bg_stroke = Stroke::new(2.0, BLUE);
    widgets.open.bg_stroke = Stroke::new(1.5, BLUE);
    widgets.noninteractive.bg_stroke = Stroke::new(1.0, theme::border());
    widgets.noninteractive.fg_stroke = Stroke::new(1.0, theme::muted());
    context.set_visuals(visuals);

    context.style_mut(|style| {
        style.animation_time = 0.14;
        style.spacing.button_padding = vec2(12.0, 7.0);
        style.spacing.interact_size.y = 32.0;
        style.spacing.scroll = egui::style::ScrollStyle::solid();
        style.spacing.scroll.bar_width = 8.0;
        style.spacing.scroll.bar_inner_margin = 5.0;
    });
}

pub(super) fn paint_surface(ui: &Ui, response: &Response, selected: bool) {
    let active = ui.is_enabled() && (response.hovered() || response.has_focus());
    let motion = settings(ui.ctx()).motion;
    let hover = ui.ctx().animate_bool_with_time(
        response.id.with("hover"),
        active,
        if motion { 0.12 } else { 0.0 },
    );
    let offset = if response.is_pointer_button_down_on() {
        1.0
    } else {
        -hover
    };
    let rect = response.rect.translate(vec2(0.0, offset));
    let radius = (rect.height() * 0.5).min(22.0) as u8;
    let painter = ui.painter();
    let edge = if selected || active {
        BLUE
    } else {
        theme::border()
    };

    painter.rect_filled(
        rect.translate(vec2(0.0, 2.0)),
        radius,
        Color32::from_black_alpha(14),
    );
    painter.rect_filled(
        rect,
        radius,
        if selected {
            Color32::from_rgb(216, 238, 249)
        } else {
            SILVER
        },
    );
    paint_gloss(painter, rect.shrink(2.0), radius as f32);
    let reflection = Rect::from_min_size(
        rect.min + vec2(3.0, 2.0),
        vec2(rect.width() - 6.0, rect.height() * 0.62),
    );
    egui::Image::new(egui::include_image!("assets/wii/button-highlight.svg"))
        .paint_at(ui, reflection);
    painter.rect_stroke(
        rect,
        radius,
        Stroke::new(1.0 + hover, edge),
        egui::StrokeKind::Inside,
    );
}

fn paint_gloss(painter: &egui::Painter, rect: Rect, radius: f32) {
    let radius = radius.min(rect.height() * 0.5).min(rect.width() * 0.5);
    let mut mesh = egui::Mesh::default();
    let color = |y: f32| {
        let fraction = ((y - rect.top()) / rect.height()).clamp(0.0, 1.0);
        egui::lerp(
            egui::Rgba::WHITE..=egui::Rgba::from(Color32::from_rgb(188, 199, 213)),
            fraction,
        )
        .into()
    };
    mesh.colored_vertex(rect.center(), color(rect.center().y));

    for (corner, start) in [
        (rect.right_top(), -std::f32::consts::FRAC_PI_2),
        (rect.right_bottom(), 0.0),
        (rect.left_bottom(), std::f32::consts::FRAC_PI_2),
        (rect.left_top(), std::f32::consts::PI),
    ] {
        let center = corner
            + vec2(
                if corner.x == rect.right() {
                    -radius
                } else {
                    radius
                },
                if corner.y == rect.top() {
                    radius
                } else {
                    -radius
                },
            );
        for step in 0..=8 {
            let angle = start + step as f32 / 8.0 * std::f32::consts::FRAC_PI_2;
            let point = center + vec2(angle.cos(), angle.sin()) * radius;
            mesh.colored_vertex(point, color(point.y));
        }
    }
    let count = mesh.vertices.len() as u32 - 1;
    for index in 1..=count {
        mesh.add_triangle(0, index, index % count + 1);
    }
    painter.add(egui::Shape::mesh(mesh));
}

pub(super) fn button(ui: &mut Ui, label: &str, width: f32, selected: bool) -> Response {
    button_with_sense(ui, label, width, selected, Sense::click())
}

pub(super) fn button_with_sense(
    ui: &mut Ui,
    label: &str,
    width: f32,
    selected: bool,
    sense: Sense,
) -> Response {
    let (rect, response) = ui.allocate_exact_size(vec2(width, 38.0), sense);
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), label)
    });
    paint_surface(ui, &response, selected);
    let color = if ui.is_enabled() {
        theme::text()
    } else {
        theme::muted()
    };
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        label,
        FontId::proportional(15.0),
        color,
    );
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

pub(super) fn striped_panel(ui: &Ui, rect: Rect) {
    ui.painter()
        .rect_filled(rect, 0, Color32::from_rgb(216, 219, 217));
    egui::Image::new(egui::include_image!("assets/wii/lines.svg")).paint_at(ui, rect);
    ui.painter()
        .hline(rect.x_range(), rect.top(), Stroke::new(1.0, Color32::WHITE));
}

pub(super) fn player_color(player: usize) -> Color32 {
    PLAYERS[player % PLAYERS.len()]
}

pub(super) fn paint_cursor(
    painter: &egui::Painter,
    tip: egui::Pos2,
    height: f32,
    player: usize,
    opacity: f32,
) {
    let source = match player % PLAYERS.len() {
        0 => egui::include_image!("assets/wii/cursor-blue.svg"),
        1 => egui::include_image!("assets/wii/cursor-red.svg"),
        2 => egui::include_image!("assets/wii/cursor-green.svg"),
        _ => egui::include_image!("assets/wii/cursor-gold.svg"),
    };
    let size = vec2(height * 85.0 / 126.0, height);
    let rect = Rect::from_min_size(tip - vec2(23.0, 5.0) * height / 126.0, size);
    let image = egui::Image::new(source).fit_to_exact_size(size);
    if let Ok(egui::load::TexturePoll::Ready { texture }) = image.load_for_size(painter.ctx(), size)
    {
        let uv = Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0));
        painter.image(texture.id, rect, uv, Color32::WHITE.gamma_multiply(opacity));
    }
}

pub(super) fn finish_frame(
    context: &egui::Context,
    sounds: bool,
    run: Option<redox::domain::RunId>,
    completed: bool,
    quiet: bool,
) {
    let state_id = egui::Id::new((FEEDBACK_KEY, context.viewport_id()));
    let mut feedback = context
        .data(|data| data.get_temp::<Feedback>(state_id))
        .unwrap_or_default();
    let (hovered_ids, clicked) = context.interaction_snapshot(|snapshot| {
        (
            snapshot.hovered.iter().copied().collect::<Vec<_>>(),
            snapshot.clicked,
        )
    });
    let hovered = hovered_ids.into_iter().find(|id| {
        context.read_response(*id).is_some_and(|response| {
            response.enabled() && response.sense.senses_click() && response.hovered()
        })
    });
    if sounds && !quiet {
        if !feedback.ready {
            play(Sound::Open);
        } else if feedback.run.is_some() && run.is_none() && completed {
            play(Sound::Success);
        } else if feedback.run.is_none() && run.is_some() {
            play(Sound::Click);
        } else if clicked.is_some_and(|id| {
            context
                .read_response(id)
                .is_some_and(|response| response.enabled() && response.clicked())
        }) {
            play(Sound::Click);
        } else if hovered.is_some() && hovered != feedback.hovered {
            play(Sound::Hover);
        }
    }
    feedback.ready |= !quiet;
    feedback.hovered = hovered;
    feedback.run = run;
    context.data_mut(|data| data.insert_temp(state_id, feedback));

    let cursor = context.output(|output| output.cursor_icon);
    let pointer = context.input(|input| input.pointer.hover_pos());
    if let Some(pointer) = pointer {
        if matches!(
            cursor,
            egui::CursorIcon::Default | egui::CursorIcon::PointingHand
        ) {
            context.set_cursor_icon(egui::CursorIcon::None);
            let painter = context.layer_painter(egui::LayerId::new(
                egui::Order::Tooltip,
                egui::Id::new("wii_pointer"),
            ));
            paint_cursor(&painter, pointer, 46.0, 0, 1.0);
        }
    }
}

impl ReDox {
    pub(super) fn wii_feedback(&self, context: &egui::Context) {
        let state_id = egui::Id::new((FEEDBACK_KEY, context.viewport_id()));
        let previous = context
            .data(|data| data.get_temp::<Feedback>(state_id))
            .and_then(|feedback| feedback.run);
        let completed = previous.is_some_and(|run| {
            self.session
                .chats
                .iter()
                .find(|chat| chat.id == run.chat)
                .and_then(|chat| {
                    chat.messages
                        .iter()
                        .find(|message| message.id == run.message)
                })
                .is_some_and(|message| message.status == redox::domain::MessageStatus::Complete)
        });
        finish_frame(
            context,
            self.ui_preferences.wii.sounds,
            self.running.as_ref().map(|run| run.id),
            completed,
            self.capture_path.is_some() || self.startup.is_active(),
        );
    }

    pub(super) fn wii_title_bar(&mut self, context: &egui::Context) {
        egui::TopBottomPanel::top("wii_title")
            .exact_height(48.0)
            .show_separator_line(false)
            .frame(
                Frame::NONE
                    .fill(theme::rail())
                    .inner_margin(Margin::symmetric(12, 7)),
            )
            .show(context, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("ReDox")
                            .size(20.0)
                            .color(theme::muted()),
                    );
                    ui.add_space(10.0);
                    self.main_menus(ui, context);
                    let (_, drag) = ui.allocate_exact_size(
                        vec2((ui.available_width() - 250.0).max(0.0), 30.0),
                        Sense::click_and_drag(),
                    );
                    super::window_frame::title_drag(context, &drag);
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if icons::button(ui, Icon::Close, "Close window").clicked() {
                            context.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                        if icons::button(ui, Icon::Maximize, "Maximize / restore").clicked() {
                            let maximized =
                                context.input(|input| input.viewport().maximized.unwrap_or(false));
                            context.send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
                        }
                        if icons::button(ui, Icon::Minimize, "Minimize").clicked() {
                            context.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                        }
                        self.title_ui_picker(ui);
                    });
                });
            });
    }

    pub(super) fn wii_rail(&mut self, context: &egui::Context) {
        context.data_mut(|data| {
            data.insert_temp(egui::Id::new(SETTINGS_KEY), self.ui_preferences.wii);
            data.insert_temp(
                egui::Id::new(QUIET_KEY),
                self.capture_path.is_some() || self.startup.is_active(),
            );
        });
        egui::TopBottomPanel::bottom("wii_dock")
            .exact_height(66.0)
            .show_separator_line(false)
            .frame(Frame::NONE.inner_margin(Margin::symmetric(18, 13)))
            .show(context, |ui| {
                striped_panel(ui, ui.max_rect().expand2(vec2(18.0, 13.0)));
                ui.horizontal(|ui| {
                    let sidebar_shown = self.sidebar_visible
                        && (!self.ui_preferences.agents_open
                            || context.viewport_rect().width() >= 1280.0);
                    if button(ui, "Chats", 94.0, sidebar_shown).clicked() {
                        self.sidebar_visible = !sidebar_shown;
                        if self.sidebar_visible && context.viewport_rect().width() < 1280.0 {
                            self.ui_preferences.agents_open = false;
                        }
                    }
                    if ui
                        .add_enabled_ui(self.can_new_chat(), |ui| {
                            button(ui, "New chat", 110.0, false)
                        })
                        .inner
                        .clicked()
                    {
                        self.new_chat();
                    }
                    if button(ui, "Open project", 126.0, false).clicked() {
                        self.pick_workspace(context);
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if button(ui, "Settings", 100.0, self.settings_open).clicked() {
                            self.settings_open = !self.settings_open;
                        }
                        if button(ui, "Agents", 94.0, self.ui_preferences.agents_open).clicked() {
                            self.ui_preferences.agents_open = !self.ui_preferences.agents_open;
                        }
                    });
                });
            });
    }

    pub(super) fn wii_conversation(&mut self, context: &egui::Context) {
        egui::TopBottomPanel::top("wii_chat_header")
            .exact_height(62.0)
            .show_separator_line(false)
            .frame(
                Frame::NONE
                    .fill(theme::background())
                    .inner_margin(Margin::symmetric(22, 12)),
            )
            .show(context, |ui| {
                ui.horizontal(|ui| {
                    for (forward, tooltip) in [(false, "Previous chat"), (true, "Next chat")] {
                        let (rect, response) =
                            ui.allocate_exact_size(vec2(26.0, 34.0), Sense::click());
                        let source = if forward {
                            egui::include_image!("assets/wii/next.svg")
                        } else {
                            egui::include_image!("assets/wii/previous.svg")
                        };
                        let uv = if forward {
                            Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0))
                        } else {
                            Rect::from_min_max(egui::pos2(1.0, 0.0), egui::pos2(0.0, 1.0))
                        };
                        egui::Image::new(source)
                            .uv(uv)
                            .paint_at(ui, Rect::from_center_size(rect.center(), vec2(13.5, 25.0)));
                        if response.on_hover_text(tooltip).clicked() {
                            self.navigate(if forward { 1 } else { -1 });
                        }
                    }
                    ui.add_space(8.0);
                    let title = self
                        .session
                        .active()
                        .map_or("New chat", |chat| chat.title.as_str());
                    let title_width = (ui.available_width() - 42.0).max(0.0);
                    ui.allocate_ui_with_layout(
                        vec2(title_width, 34.0),
                        Layout::left_to_right(Align::Center),
                        |ui| {
                            ui.add(
                                egui::Label::new(egui::RichText::new(title).size(20.0)).truncate(),
                            );
                        },
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if icons::button(ui, Icon::More, "Rename chat").clicked() {
                            self.rename_open = true;
                        }
                    });
                });
            });
        self.composer(context, None);

        egui::CentralPanel::default()
            .frame(
                Frame::NONE
                    .fill(theme::background())
                    .inner_margin(Margin::symmetric(26, 4)),
            )
            .show(context, |ui| {
                let Some(chat) = self.session.active() else {
                    return;
                };
                if let Some(texture) = &self.ui_background {
                    super::skins::paint_background(
                        ui.painter(),
                        ui.max_rect(),
                        texture,
                        self.active_ui.config.background_opacity,
                    );
                }
                if !chat.messages.is_empty() {
                    render_transcript(ui, chat, chat.id, 0.0, self.ui_preferences.codex_mode);
                    return;
                }

                if let Some(action) = home_channels(ui, &self.active_ui.config.heading) {
                    action.activate(self, context);
                }
            });
    }
}

#[derive(Clone, Copy)]
enum Shortcut {
    Project,
    Search,
    Agents,
    Settings,
}

const SHORTCUTS: [(Shortcut, &str, Icon); 4] = [
    (Shortcut::Project, "Open project", Icon::Folder),
    (Shortcut::Search, "Search chats", Icon::Search),
    (Shortcut::Agents, "Ollama agents", Icon::Agent),
    (Shortcut::Settings, "Settings", Icon::Settings),
];

impl Shortcut {
    fn activate(self, app: &mut ReDox, context: &egui::Context) {
        match self {
            Self::Project => app.pick_workspace(context),
            Self::Search => {
                app.search_open = true;
                app.sidebar_visible = true;
                if context.viewport_rect().width() < 1280.0 {
                    app.ui_preferences.agents_open = false;
                }
            }
            Self::Agents => app.ui_preferences.agents_open = true,
            Self::Settings => app.settings_open = true,
        }
    }
}

fn home_channels(ui: &mut Ui, heading: &str) -> Option<Shortcut> {
    let available = ui.available_size();
    let columns = if available.x >= 390.0 { 2 } else { 1 };
    let rows = SHORTCUTS.len().div_ceil(columns);
    let top = ((available.y - 400.0) * 0.3).clamp(0.0, 110.0);
    let tile_height = ((available.y - top - 78.0) / rows as f32 - 14.0).max(90.0);
    let tile_width = ((available.x - 12.0 * (columns - 1) as f32) / columns as f32)
        .min(tile_height * 392.0 / 217.0)
        .min(244.0);
    let width = tile_width * columns as f32 + 12.0 * (columns - 1) as f32;
    let mut selected = None;

    egui::ScrollArea::vertical()
        .id_salt("wii_home_channels")
        .show(ui, |ui| {
            ui.add_space(top);
            ui.vertical_centered(|ui| {
                ui.label(egui::RichText::new(heading).size(28.0));
                ui.add_space(28.0);
                ui.allocate_ui_with_layout(vec2(width, 0.0), Layout::top_down(Align::Min), |ui| {
                    for row in SHORTCUTS.chunks(columns) {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 12.0;
                            for (action, label, icon) in row {
                                if channel(ui, label, *icon, tile_width).clicked() {
                                    selected = Some(*action);
                                    ui.ctx().request_repaint_of(egui::ViewportId::ROOT);
                                }
                            }
                        });
                        ui.add_space(6.0);
                    }
                });
            });
        });
    selected
}

fn channel(ui: &mut Ui, label: &str, icon: Icon, width: f32) -> Response {
    let height = width * 217.0 / 392.0;
    let (rect, response) = ui.allocate_exact_size(vec2(width, height), Sense::click());
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), label)
    });
    let motion = settings(ui.ctx()).motion;
    let hover = ui.ctx().animate_bool_with_time(
        response.id.with("channel_hover"),
        response.hovered() || response.has_focus(),
        if motion { 0.16 } else { 0.0 },
    );
    let bounds = rect.shrink(2.0).translate(vec2(0.0, -2.0 * hover));
    egui::Image::new(egui::include_image!("assets/wii/channel.svg")).paint_at(ui, bounds);
    ui.painter()
        .rect_filled(bounds.shrink(4.0), 18, Color32::from_rgb(249, 251, 252));
    if hover > 0.0 {
        ui.painter().rect_stroke(
            bounds.shrink(1.0),
            19,
            Stroke::new(2.0, BLUE.gamma_multiply(hover)),
            egui::StrokeKind::Inside,
        );
    }
    icons::paint(
        ui.painter(),
        Rect::from_center_size(bounds.center() - vec2(0.0, 14.0), vec2(34.0, 34.0)),
        icon,
        Color32::from_rgb(66, 159, 201),
    );
    ui.painter().text(
        bounds.center() + vec2(0.0, 26.0),
        Align2::CENTER_CENTER,
        label,
        FontId::proportional(16.0),
        theme::text(),
    );
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

#[cfg(test)]
#[path = "../../tests/ui/wii.rs"]
mod tests;
