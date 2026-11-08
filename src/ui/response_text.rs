use std::collections::HashMap;
use std::ops::Range;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use eframe::egui::{self, Rect, Sense, Shape, epaint::TextShape};
use redox::domain::Message;

const ACCENT_DURATION: Duration = Duration::from_secs(2);

#[derive(Clone, Default)]
struct VisibleNumbers {
    appearances: HashMap<(i32, i32, usize), Appearance>,
}

#[derive(Clone, Copy)]
struct Appearance {
    frame: u64,
    started_at: f64,
}

pub(super) fn show(ui: &mut egui::Ui, message: &Message, workspace: Option<&Path>) {
    show_text(ui, message.id, &message.content, workspace);
}

pub(super) fn show_text(
    ui: &mut egui::Ui,
    message_id: uuid::Uuid,
    source: &str,
    workspace: Option<&Path>,
) {
    let accent = source.contains("67");
    let tooltip = source.contains("727");
    if !accent && !tooltip {
        super::markdown::show_in_workspace(ui, source, workspace);
        return;
    }

    let context = ui.ctx().clone();
    let layer = ui.layer_id();
    let origin = ui.next_widget_position();
    let start = context.graphics(|layers| layers.get(layer).map_or(0, |list| list.next_idx().0));
    super::markdown::show_in_workspace(ui, source, workspace);
    let shapes: Vec<_> = context.graphics(|layers| {
        layers
            .get(layer)
            .into_iter()
            .flat_map(|list| list.all_entries().enumerate().skip(start))
            .filter_map(|(index, entry)| match &entry.shape {
                Shape::Text(text) => Some((index, entry.clip_rect, text.clone())),
                _ => None,
            })
            .collect()
    });

    let id = ui
        .id()
        .with(("response_text", context.viewport_id(), message_id));
    let frame = context.cumulative_frame_nr();
    let now = ui.input(|input| input.time);
    if accent {
        context.data_mut(|data| {
            data.get_temp_mut_or_default::<VisibleNumbers>(id)
                .appearances
                .retain(|_, appearance| appearance.frame >= frame.saturating_sub(1));
        });
    }
    for (index, clip, mut text) in shapes {
        let label = text.galley.text();
        let mut motion = Vec::new();
        if accent {
            for bytes in number_ranges(label, "67") {
                if !glyph_rects(&text, bytes.clone()).any(|rect| rect.intersect(clip).is_positive())
                {
                    continue;
                }
                let position = text.pos - origin;
                let key = (
                    (position.x * 4.0).round() as i32,
                    (position.y * 4.0).round() as i32,
                    bytes.start,
                );
                let elapsed = context.data_mut(|data| {
                    let appearance = data
                        .get_temp_mut_or_default::<VisibleNumbers>(id)
                        .appearances
                        .entry(key)
                        .or_insert(Appearance {
                            frame,
                            started_at: now,
                        });
                    appearance.frame = frame;
                    now - appearance.started_at
                });
                if elapsed < ACCENT_DURATION.as_secs_f64() {
                    let progress = (elapsed / ACCENT_DURATION.as_secs_f64()) as f32;
                    let angle =
                        14.0_f32.to_radians() * (progress * std::f32::consts::TAU * 3.0).sin();
                    motion.push((bytes, angle));
                    context.request_repaint_after(Duration::from_millis(16));
                }
            }
        }
        if tooltip {
            for bytes in number_ranges(label, "727") {
                let offset = bytes.start;
                for (row, rect) in glyph_rects(&text, bytes).enumerate() {
                    let rect = clip.intersect(rect);
                    if rect.is_positive() {
                        ui.interact(
                            rect,
                            ui.id().with((id, index - start, offset, row)),
                            Sense::hover(),
                        )
                        .on_hover_text("WYSI");
                    }
                }
            }
        }
        if !motion.is_empty() {
            for (bytes, angle) in motion {
                rotate_glyphs(&mut text, bytes, angle);
            }
            context.graphics_mut(|layers| {
                if let Some(list) = layers.get_mut(layer) {
                    list.mutate_shape(egui::layers::ShapeIdx(index), |entry| {
                        entry.shape = Shape::Text(text);
                    });
                }
            });
        }
    }
}

#[cfg(test)]
#[path = "../../tests/ui/response_text.rs"]
mod tests;

fn number_ranges<'a>(text: &'a str, number: &'a str) -> impl Iterator<Item = Range<usize>> + 'a {
    let joins_word = |ch: char| ch.is_alphanumeric() || ch == '_';
    text.match_indices(number).filter_map(move |(start, _)| {
        let end = start + number.len();
        (!text[..start].chars().next_back().is_some_and(joins_word)
            && !text[end..].chars().next().is_some_and(joins_word))
        .then_some(start..end)
    })
}

fn glyph_rects(text: &TextShape, bytes: Range<usize>) -> impl Iterator<Item = Rect> + '_ {
    glyph_spans(text, bytes).map(|(_, _, rect)| rect)
}

fn glyph_spans(
    text: &TextShape,
    bytes: Range<usize>,
) -> impl Iterator<Item = (usize, Range<usize>, Rect)> + '_ {
    let start = text.galley.text()[..bytes.start].chars().count();
    let end = start + bytes.len();
    let mut offset = 0;
    text.galley
        .rows
        .iter()
        .enumerate()
        .map_while(move |(index, row)| {
            if offset >= end {
                return None;
            }
            let first = start.saturating_sub(offset);
            let last = end.saturating_sub(offset).min(row.glyphs.len());
            offset += row.glyphs.len() + usize::from(row.ends_with_newline);
            Some((first < last).then(|| {
                let rect = Rect::from_min_max(
                    egui::pos2(row.x_offset(first), row.rect().top()),
                    egui::pos2(row.x_offset(last), row.rect().bottom()),
                )
                .translate(text.pos.to_vec2());
                (index, first..last, rect)
            }))
        })
        .flatten()
}

fn rotate_glyphs(text: &mut TextShape, bytes: Range<usize>, angle: f32) {
    let spans: Vec<_> = glyph_spans(text, bytes).collect();
    let rotation = egui::emath::Rot2::from_angle(angle);
    let color = super::theme::blue();
    let galley = Arc::make_mut(&mut text.galley);
    if let Some(original) = text.override_text_color.take() {
        for row in &mut galley.rows {
            let visuals = &mut Arc::make_mut(&mut row.row).visuals;
            for vertex in &mut visuals.mesh.vertices[visuals.glyph_vertex_range.clone()] {
                vertex.color = original;
            }
        }
    }
    for (index, columns, rect) in spans {
        let row = &mut galley.rows[index];
        let center = rect.center() - text.pos.to_vec2() - row.pos.to_vec2();
        let row = Arc::make_mut(&mut row.row);
        let start = row.glyphs[columns.start].first_vertex as usize;
        let end = row
            .glyphs
            .get(columns.end)
            .map_or(row.visuals.glyph_vertex_range.end, |glyph| {
                glyph.first_vertex as usize
            });
        for vertex in &mut row.visuals.mesh.vertices[start..end] {
            vertex.pos = center + rotation * (vertex.pos - center);
            vertex.color = color;
        }
        row.visuals.mesh_bounds = row.visuals.mesh.calc_bounds();
    }
    galley.mesh_bounds = galley.rows.iter().fold(Rect::NOTHING, |bounds, row| {
        bounds.union(row.visuals.mesh_bounds.translate(row.pos.to_vec2()))
    });
}
