use crate::{
    actions::UiAction,
    document::{Document, Kind, Node, Page, TextAlign},
    presentation::scene::{self, View},
};
use eframe::egui::{self, Color32, FontId, Response, Sense, Vec2};

#[derive(Default)]
pub struct Values<'a> {
    pub model: Option<&'a str>,
    pub effort: Option<&'a str>,
    pub model_effort: Option<&'a str>,
}

pub struct Control<'a> {
    pub action: UiAction,
    pub node: &'a Node,
    pub response: Response,
    pub zoom: f32,
    pub font: FontId,
    pub text_color: Color32,
    pub placeholder: &'a str,
    pub text_align: TextAlign,
}

pub struct ComposerResult {
    pub response: Response,
    pub submitted: bool,
}

/// Lay out live content inside its authored bounds without expanding the scene.
pub fn content<R>(
    ui: &mut egui::Ui,
    control: &Control<'_>,
    contents: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    let rect = control.response.rect;
    let padding = (control.node.padding * control.zoom)
        .min(((rect.width().min(rect.height()) - 1.) * 0.5).max(0.));
    let bounds = rect.shrink(padding);
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .id_salt(("content", control.response.id))
            .max_rect(bounds)
            .layout(egui::Layout::top_down(match control.text_align {
                TextAlign::Left => egui::Align::Min,
                TextAlign::Center => egui::Align::Center,
                TextAlign::Right => egui::Align::Max,
            })),
    );
    child.set_clip_rect(ui.clip_rect().intersect(bounds));
    if control.node.fill[3] > 0 {
        child.style_mut().visuals = if control.node.fill[..3]
            .iter()
            .map(|c| u32::from(*c))
            .sum::<u32>()
            > 384
        {
            egui::Visuals::light()
        } else {
            egui::Visuals::dark()
        };
    }
    child.style_mut().visuals.override_text_color = Some(control.text_color);
    child.style_mut().override_font_id = Some(control.font.clone());
    contents(&mut child)
}

/// Shared keyboard and typography behavior for both the sandbox and the real host.
pub fn composer(
    ui: &mut egui::Ui,
    control: &Control<'_>,
    draft: &mut String,
    session: egui::Id,
    focus: bool,
) -> ComposerResult {
    let id = control.response.id.with(("composer", session));
    if focus {
        ui.memory_mut(|m| m.request_focus(id));
    }
    let inset = if control.node.kind == Kind::Input {
        12. * control.zoom
    } else {
        0.
    };
    let response = ui.add_sized(
        control.response.rect.size(),
        egui::TextEdit::multiline(draft)
            .id(id)
            .frame(false)
            .margin(egui::vec2(inset, 0.))
            .font(control.font.clone())
            .text_color(control.text_color)
            .horizontal_align(match control.text_align {
                TextAlign::Left => egui::Align::Min,
                TextAlign::Center => egui::Align::Center,
                TextAlign::Right => egui::Align::Max,
            })
            .vertical_align(if control.node.kind == Kind::Input {
                egui::Align::Center
            } else {
                egui::Align::Min
            })
            .hint_text(
                egui::RichText::new(if control.placeholder.is_empty() {
                    "Ask anything"
                } else {
                    control.placeholder
                })
                .color(control.text_color)
                .font(control.font.clone()),
            )
            .return_key(egui::KeyboardShortcut::new(
                egui::Modifiers::SHIFT,
                egui::Key::Enter,
            )),
    );
    let focused = response.has_focus();
    let ime_id = id.with("composition");
    let mut composing = ui.data(|d| d.get_temp::<bool>(ime_id)).unwrap_or(false);
    let (has_ime, enter) = ui.input(|input| {
        let mut has_ime = false;
        let mut enter = false;
        for event in &input.events {
            match event {
                egui::Event::Ime(event) if focused => {
                    has_ime |= matches!(
                        event,
                        egui::ImeEvent::Preedit(_) | egui::ImeEvent::Commit(_)
                    ) || (composing && matches!(event, egui::ImeEvent::Disabled));
                    composing = match event {
                        egui::ImeEvent::Enabled => composing,
                        egui::ImeEvent::Preedit(text) => !text.is_empty(),
                        egui::ImeEvent::Commit(_) | egui::ImeEvent::Disabled => false,
                    };
                }
                egui::Event::Key {
                    key: egui::Key::Enter,
                    pressed: true,
                    repeat: false,
                    modifiers,
                    ..
                } if *modifiers == egui::Modifiers::NONE => enter = true,
                _ => {}
            }
        }
        (has_ime, enter)
    });
    if !focused {
        composing = false;
    }
    ui.data_mut(|d| d.insert_temp(ime_id, composing));
    let submitted = focused && enter && !has_ime && !composing;
    ComposerResult {
        response,
        submitted,
    }
}

/// Keep the authored control as the only trigger; native UI lives in its popup.
pub fn show(
    ui: &mut egui::Ui,
    document: &Document,
    native: &mut impl FnMut(&mut egui::Ui, UiAction),
) -> Vec<UiAction> {
    show_with_values(ui, document, Values::default(), &mut |ui, control| {
        if matches!(control.action, UiAction::Composer | UiAction::Conversation) {
            native(ui, control.action);
        } else {
            egui::Popup::menu(&control.response).show(|ui| native(ui, control.action));
        }
    })
}

pub fn selected_page(
    document: &Document,
    page_index: usize,
    frame: Option<u32>,
) -> Result<Document, String> {
    let mut page = document
        .pages
        .get(page_index)
        .ok_or("Preview page is missing")?
        .clone();
    if let Some(id) = frame {
        let bounds = page.world_rect(id).ok_or("Preview frame is missing")?;
        if page.node(id).is_none_or(|n| n.kind != Kind::Frame) {
            return Err("Select a frame to preview".into());
        }
        let keep: std::collections::HashSet<_> = page
            .nodes
            .iter()
            .filter(|n| n.id == id || page.ancestors(n.id).contains(&id))
            .map(|n| n.id)
            .collect();
        page.nodes.retain(|n| keep.contains(&n.id));
        let root = page.node_mut(id).unwrap();
        root.parent = None;
        root.x = bounds[0];
        root.y = bounds[1];
    }
    let mut selected = document.clone();
    selected.pages = vec![page];
    Ok(selected)
}

fn label_node<'a>(page: &'a Page, node: &'a Node) -> &'a Node {
    if !node.text.is_empty() {
        return node;
    }
    let Some(bounds) = page.world_rect(node.id) else {
        return node;
    };
    let rect = egui::Rect::from_min_size(
        egui::pos2(bounds[0], bounds[1]),
        egui::vec2(bounds[2], bounds[3]),
    );
    page.nodes
        .iter()
        .find(|candidate| {
            candidate.kind == Kind::Text
                && !candidate.text.is_empty()
                && scene::visible(page, candidate.id)
                && page.world_rect(candidate.id).is_some_and(|b| {
                    rect.contains(egui::pos2(b[0] + b[2] * 0.5, b[1] + b[3] * 0.5))
                })
        })
        .unwrap_or(node)
}

pub fn show_with_values(
    ui: &mut egui::Ui,
    document: &Document,
    values: Values<'_>,
    native: &mut impl FnMut(&mut egui::Ui, &Control<'_>),
) -> Vec<UiAction> {
    let Some(page) = document.pages.first() else {
        return vec![];
    };
    let Some(bounds) = crate::editor::union(
        page.nodes
            .iter()
            .filter(|n| n.parent.is_none() && n.visible)
            .filter_map(|n| page.world_rect(n.id)),
    ) else {
        return vec![];
    };
    let zoom = (ui.available_width().max(1.) / bounds[2].max(1.)).min(2.0);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(bounds[2], bounds[3]) * zoom, Sense::hover());
    let view = View {
        origin: rect.min - Vec2::new(bounds[0], bounds[1]) * zoom,
        zoom,
    };
    let mut painting = page.clone();
    for node in &page.nodes {
        let [action] = node.ui_script.as_slice() else {
            continue;
        };
        if !action.is_control()
            || !scene::visible(page, node.id)
            || scene::opacity(page, node.id) <= 0.
        {
            continue;
        }
        if matches!(action, UiAction::Composer | UiAction::Conversation) {
            let region = view.rect(page.world_rect(node.id).unwrap());
            for label in &mut painting.nodes {
                if label.id == node.id
                    && (label.kind.has_text() || *action == UiAction::Conversation)
                {
                    label.text.clear();
                }
                if *action == UiAction::Composer
                    && label.kind == Kind::Text
                    && view
                        .rect(page.world_rect(label.id).unwrap())
                        .center()
                        .is_finite()
                    && region.contains(view.rect(page.world_rect(label.id).unwrap()).center())
                {
                    label.visible = false;
                }
            }
        } else if let Some(value) = match action {
            UiAction::Model => values.model,
            UiAction::Effort => values.effort,
            UiAction::ModelEffort => values.model_effort,
            _ => None,
        } {
            if !node.text.is_empty() {
                painting.node_mut(node.id).unwrap().text = value.into();
            } else {
                let region = view.rect(page.world_rect(node.id).unwrap());
                let style = label_node(page, node);
                for label in &mut painting.nodes {
                    if label.kind == Kind::Text
                        && region.contains(view.rect(page.world_rect(label.id).unwrap()).center())
                    {
                        label.visible = false;
                    }
                }
                let label = painting.node_mut(node.id).unwrap();
                label.text = value.into();
                label.font = style.font;
                label.font_size = style.font_size;
                label.text_color = style.text_color;
            }
        }
    }
    scene::render(
        &ui.painter_at(rect),
        &painting,
        view,
        None,
        &document.textures,
    );
    let mut calls = vec![];
    for id in page.scene_order() {
        let node = page.node(id).unwrap();
        if node.ui_script.is_empty() || !scene::visible(page, id) {
            continue;
        }
        let opacity = scene::opacity(page, id);
        if opacity <= 0. {
            continue;
        }
        let control_rect = view.rect(page.world_rect(id).unwrap());
        let clip = scene::clip_rect(page, id, view, rect)
            .intersect(ui.clip_rect())
            .intersect(control_rect);
        if !clip.is_positive() {
            continue;
        }
        let action = node.ui_script[0];
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(control_rect).id_salt((
            "skin_control",
            ui.ctx().viewport_id(),
            &document.name,
            id,
            action.id(),
        )));
        child.set_clip_rect(clip);
        child.set_opacity(opacity);
        let sense = if matches!(action, UiAction::Composer | UiAction::Conversation) {
            Sense::hover()
        } else {
            Sense::click()
        };
        let response = child.interact(control_rect, child.id().with("trigger"), sense);
        let response = if matches!(action, UiAction::Composer | UiAction::Conversation) {
            response
        } else {
            response.on_hover_cursor(egui::CursorIcon::PointingHand)
        };
        if !matches!(action, UiAction::Composer | UiAction::Conversation) {
            response.widget_info(|| {
                egui::WidgetInfo::labeled(
                    egui::WidgetType::Button,
                    child.is_enabled(),
                    if node.text.is_empty() {
                        action.label()
                    } else {
                        &node.text
                    },
                )
            });
        }
        if response.has_focus() && !matches!(action, UiAction::Composer | UiAction::Conversation) {
            child.painter().rect_stroke(
                control_rect.shrink(1.),
                (node.radius * zoom).max(2.),
                egui::Stroke::new(
                    1.5_f32,
                    scene::color(label_node(page, node).text_color, opacity),
                ),
                egui::StrokeKind::Inside,
            );
        }
        if action.is_control() {
            let label = if action == UiAction::Conversation {
                node
            } else {
                label_node(page, node)
            };
            let control = Control {
                action,
                node,
                response,
                zoom,
                font: scene::font(label, zoom, ui.ctx()),
                text_color: scene::color(label.text_color, 1.),
                placeholder: &label.text,
                text_align: label.text_align,
            };
            native(&mut child, &control);
        } else if response.clicked() {
            calls = node.ui_script.clone();
        }
    }
    ui.advance_cursor_after_rect(rect);
    calls
}
