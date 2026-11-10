use eframe::egui::{self, Color32, Id, Rect, Sense, Stroke, Ui, Vec2, pos2, vec2};

const WIDTH: f32 = 17.0;
const SILVER: Color32 = Color32::from_gray(192);

pub(super) struct ScrollArea {
    area: egui::ScrollArea,
    axis: usize,
    max_size: Vec2,
    salt: Option<Id>,
}

impl ScrollArea {
    pub fn vertical() -> Self {
        Self {
            area: egui::ScrollArea::vertical(),
            axis: 1,
            max_size: Vec2::INFINITY,
            salt: None,
        }
    }

    pub fn horizontal() -> Self {
        Self {
            area: egui::ScrollArea::horizontal(),
            axis: 0,
            max_size: Vec2::INFINITY,
            salt: None,
        }
    }

    pub fn id_salt(mut self, salt: impl std::hash::Hash) -> Self {
        self.area = self.area.id_salt(&salt);
        self.salt = Some(Id::new(salt));
        self
    }

    pub fn max_height(mut self, height: f32) -> Self {
        self.max_size.y = height;
        self.area = self.area.max_height(height);
        self
    }

    pub fn max_width(mut self, width: f32) -> Self {
        self.max_size.x = width;
        self.area = self.area.max_width(width);
        self
    }

    pub fn auto_shrink(mut self, shrink: [bool; 2]) -> Self {
        self.area = self.area.auto_shrink(shrink);
        self
    }

    pub fn horizontal_scroll_offset(mut self, offset: f32) -> Self {
        self.area = self.area.horizontal_scroll_offset(offset);
        self
    }

    pub fn stick_to_bottom(mut self, stick: bool) -> Self {
        self.area = self.area.stick_to_bottom(stick);
        self
    }

    pub fn show<R>(
        mut self,
        ui: &mut Ui,
        contents: impl FnOnce(&mut Ui) -> R,
    ) -> egui::scroll_area::ScrollAreaOutput<R> {
        if !super::theme::is_classic(ui) {
            return self.area.show(ui, contents);
        }
        let id = ui.make_persistent_id(
            self.salt
                .unwrap_or_else(|| ui.next_auto_id().with("classic_scroll")),
        );
        let offset = ui.data_mut(|data| data.remove_temp::<Vec2>(id.with("classic_offset")));
        self.area = self.area.id_salt(id);
        if let Some(offset) = offset {
            self.area = self.area.scroll_offset(offset).stick_to_bottom(false);
        }
        let axis = self.axis;
        let mut bounds = ui.available_rect_before_wrap();
        bounds.max = bounds.min + bounds.size().min(self.max_size);
        let arrow = WIDTH.min(bounds.size()[axis] / 2.0);
        let mut track_bounds = bounds;
        track_bounds.min[axis] += arrow;
        track_bounds.max[axis] -= arrow;
        self.area = self
            .area
            .scroll_bar_rect(track_bounds)
            .scroll_bar_visibility(if axis == 1 {
                egui::scroll_area::ScrollBarVisibility::AlwaysVisible
            } else {
                egui::scroll_area::ScrollBarVisibility::VisibleWhenNeeded
            });
        let mut output = ui
            .scope(|ui| {
                ui.spacing_mut().scroll = egui::style::ScrollStyle {
                    bar_width: WIDTH,
                    bar_inner_margin: 0.0,
                    bar_outer_margin: 0.0,
                    handle_min_length: 17.0,
                    ..egui::style::ScrollStyle::solid()
                };
                self.area.show(ui, contents)
            })
            .inner;
        let max_offset = (output.content_size[axis] - output.inner_rect.size()[axis]).max(0.0);
        if axis == 0 && max_offset <= 0.0 {
            return output;
        }
        let mut bar = output.inner_rect;
        bar.min[1 - axis] = bar.max[1 - axis];
        bar.max[1 - axis] += WIDTH;
        let (first, track, last, thumb) = geometry(
            bar,
            axis,
            output.inner_rect.size()[axis],
            output.content_size[axis],
            output.state.offset[axis],
        );
        paint_track(ui, track);
        let mut delta = 0.0;
        for (end, rect, direction) in [(0, first, -1.0), (1, last, 1.0)] {
            let response = ui.interact(
                rect,
                output.id.with(("classic_arrow", end)),
                if max_offset > 0.0 {
                    Sense::click()
                } else {
                    Sense::hover()
                },
            );
            let held = response.is_pointer_button_down_on();
            paint_button(ui, rect, held);
            paint_arrow(ui, rect, axis, direction, held, max_offset > 0.0);
            if response.clicked() {
                delta += direction * 24.0;
            } else if held {
                let now = ui.input(|input| input.time);
                let repeat_id = response.id.with("repeat");
                let next =
                    ui.data_mut(|data| *data.get_temp_mut_or_insert_with(repeat_id, || now + 0.35));
                if now >= next {
                    delta += direction * 24.0;
                    ui.data_mut(|data| data.insert_temp(repeat_id, now + 0.06));
                }
                ui.ctx().request_repaint();
            } else {
                ui.data_mut(|data| data.remove::<f64>(response.id.with("repeat")));
            }
        }
        if max_offset > 0.0 {
            paint_button(ui, thumb, false);
        }
        if delta != 0.0 {
            output.state.offset[axis] = (output.state.offset[axis] + delta).clamp(0.0, max_offset);
            output.state.store(ui.ctx(), output.id);
            ui.data_mut(|data| data.insert_temp(id.with("classic_offset"), output.state.offset));
            ui.ctx().request_repaint();
        }
        output
    }
}

fn geometry(
    bar: Rect,
    axis: usize,
    visible: f32,
    content: f32,
    offset: f32,
) -> (Rect, Rect, Rect, Rect) {
    let arrow = WIDTH.min(bar.size()[axis] / 2.0);
    let mut first = bar;
    first.max[axis] = first.min[axis] + arrow;
    let mut last = bar;
    last.min[axis] = last.max[axis] - arrow;
    let mut track = bar;
    track.min[axis] += arrow;
    track.max[axis] -= arrow;
    let length = track.size()[axis];
    let thumb_length = (length * visible / content.max(1.0)).max(17.0).min(length);
    let fraction = (offset / (content - visible).max(1.0)).clamp(0.0, 1.0);
    let mut thumb = track;
    thumb.min[axis] += fraction * (length - thumb_length);
    thumb.max[axis] = thumb.min[axis] + thumb_length;
    (first, track, last, thumb)
}

fn paint_track(ui: &Ui, rect: Rect) {
    let painter = ui.painter().with_clip_rect(ui.clip_rect().intersect(rect));
    painter.rect_filled(rect, 0, Color32::WHITE);
    let mut mesh = egui::Mesh::default();
    let pixels = ui.ctx().pixels_per_point();
    let left = (rect.left() * pixels).floor() as i32;
    let top = (rect.top() * pixels).floor() as i32;
    let right = (rect.right() * pixels).ceil() as i32;
    let bottom = (rect.bottom() * pixels).ceil() as i32;
    for y in top..bottom {
        for x in left..right {
            if (x + y) % 2 == 0 {
                mesh.add_colored_rect(
                    Rect::from_min_size(
                        pos2(x as f32 / pixels, y as f32 / pixels),
                        Vec2::splat(1.0 / pixels),
                    ),
                    SILVER,
                );
            }
        }
    }
    painter.add(egui::Shape::mesh(mesh));
}

fn paint_button(ui: &Ui, rect: Rect, pressed: bool) {
    ui.painter().rect_filled(rect, 0, SILVER);
    super::win98::bevel(ui.painter(), rect, pressed);
    if !pressed {
        let inner = rect.shrink(1.5);
        ui.painter().hline(
            inner.x_range(),
            inner.bottom(),
            Stroke::new(1.0, Color32::GRAY),
        );
        ui.painter().vline(
            inner.right(),
            inner.y_range(),
            Stroke::new(1.0, Color32::GRAY),
        );
    }
}

fn paint_arrow(ui: &Ui, rect: Rect, axis: usize, direction: f32, pressed: bool, enabled: bool) {
    let center = rect.center() + Vec2::splat(if pressed { 1.0 } else { 0.0 });
    for row in 0..4 {
        let mut point = center;
        point[axis] += direction * (1.5 - row as f32);
        point[1 - axis] -= row as f32;
        let size = if axis == 1 {
            vec2((row * 2 + 1) as f32, 1.0)
        } else {
            vec2(1.0, (row * 2 + 1) as f32)
        };
        ui.painter().rect_filled(
            Rect::from_min_size(point, size),
            0,
            if enabled {
                Color32::BLACK
            } else {
                Color32::GRAY
            },
        );
    }
}

#[cfg(test)]
#[path = "../../tests/ui/scroll.rs"]
mod tests;
