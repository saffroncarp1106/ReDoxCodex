use crate::document::{Font, Kind, Node, Page, TextAlign};
use eframe::egui::{self, Color32, FontFamily, FontId, Pos2, Rect, Stroke, StrokeKind, Vec2};

const MAX_CACHED_TEXTURES: usize = 32;
const MAX_CACHED_PIXELS: usize = 16 * 1024 * 1024;

#[derive(Clone)]
struct CachedTexture {
    key: u64,
    handle: egui::TextureHandle,
    used: u64,
}

#[derive(Clone, Default)]
struct TextureCache {
    textures: Vec<CachedTexture>,
}

impl TextureCache {
    fn prune(&mut self, frame: u64) {
        let mut pixels: usize = self
            .textures
            .iter()
            .map(|t| t.handle.size().iter().product::<usize>())
            .sum();
        while self.textures.len() > MAX_CACHED_TEXTURES || pixels > MAX_CACHED_PIXELS {
            // An earlier render in this frame can still refer to these texture IDs.
            let Some((index, _)) = self
                .textures
                .iter()
                .enumerate()
                .filter(|(_, t)| t.used < frame)
                .min_by_key(|(_, t)| t.used)
            else {
                break;
            };
            let removed = self.textures.remove(index);
            pixels -= removed.handle.size().iter().product::<usize>();
        }
    }
}

#[derive(Clone, Copy)]
pub struct View {
    pub origin: Pos2,
    pub zoom: f32,
}

impl View {
    pub fn screen(self, point: Pos2) -> Pos2 {
        self.origin + point.to_vec2() * self.zoom
    }
    pub fn world(self, point: Pos2) -> Pos2 {
        Pos2::ZERO + (point - self.origin) / self.zoom
    }
    pub fn rect(self, bounds: [f32; 4]) -> Rect {
        Rect::from_min_size(
            self.screen(Pos2::new(bounds[0], bounds[1])),
            Vec2::new(bounds[2], bounds[3]) * self.zoom,
        )
    }
}

pub fn color(rgba: [u8; 4], opacity: f32) -> Color32 {
    Color32::from_rgba_unmultiplied(rgba[0], rgba[1], rgba[2], (rgba[3] as f32 * opacity) as u8)
}

pub fn font(node: &Node, zoom: f32, ctx: &egui::Context) -> FontId {
    let source = node
        .artwork
        .as_ref()
        .filter(|art| node.font == art.original_font)
        .and_then(|art| art.font_face.as_ref())
        .map(|name| FontFamily::Name(name.clone().into()))
        .filter(|family| ctx.fonts(|f| f.families().contains(family)));
    FontId::new(
        node.font_size * zoom,
        source.unwrap_or(if node.font == Font::Mono {
            FontFamily::Monospace
        } else {
            FontFamily::Proportional
        }),
    )
}

pub fn visible(page: &Page, id: u32) -> bool {
    page.node(id).is_some_and(|n| n.visible)
        && page
            .ancestors(id)
            .iter()
            .all(|id| page.node(*id).is_some_and(|n| n.visible))
}

pub fn opacity(page: &Page, id: u32) -> f32 {
    page.node(id).map_or(0., |node| {
        page.ancestors(id)
            .iter()
            .fold(node.opacity, |alpha, parent| {
                alpha * page.node(*parent).map_or(0., |node| node.opacity)
            })
    })
}

pub fn clip_rect(page: &Page, id: u32, view: View, viewport: Rect) -> Rect {
    let mut clip = viewport;
    for id in page.ancestors(id) {
        if page.node(id).is_some_and(|n| n.clip_content)
            && let Some(bounds) = page.world_rect(id)
        {
            clip = clip.intersect(view.rect(bounds));
        }
    }
    clip
}

pub fn hit(page: &Page, world: Pos2, containers_only: bool) -> Option<u32> {
    page.scene_order().into_iter().rev().find(|id| {
        let node = page.node(*id).unwrap();
        if !page.editable(*id) || (containers_only && !node.kind.container()) {
            return false;
        }
        let r = page.world_rect(*id).unwrap();
        let rect = Rect::from_min_size(Pos2::new(r[0], r[1]), Vec2::new(r[2], r[3]));
        let inside = if node.kind == Kind::Ellipse {
            let delta = (world - rect.center()) / (rect.size() * 0.5);
            delta.length_sq() <= 1.
        } else {
            rect.expand(if node.kind == Kind::Line { 4. } else { 0. })
                .contains(world)
        };
        inside
            && page.ancestors(*id).iter().all(|parent| {
                let node = page.node(*parent).unwrap();
                if !node.clip_content {
                    return true;
                }
                let r = page.world_rect(*parent).unwrap();
                Rect::from_min_size(Pos2::new(r[0], r[1]), Vec2::new(r[2], r[3])).contains(world)
            })
    })
}

pub fn render(
    painter: &egui::Painter,
    page: &Page,
    view: View,
    editing: Option<u32>,
    assets: &[crate::document::TextureAsset],
) {
    use base64::Engine;
    use std::hash::{Hash, Hasher};
    // Viewports paint independently, so each owns the lifetime of its cached textures.
    let cache_id = egui::Id::new(("skin_textures", painter.ctx().viewport_id()));
    let frame = painter.ctx().cumulative_frame_nr();
    let mut cache = painter
        .ctx()
        .data(|d| d.get_temp::<TextureCache>(cache_id))
        .unwrap_or_default();
    let textures: Vec<_> = assets
        .iter()
        .map(|asset| {
            let mut hash = std::collections::hash_map::DefaultHasher::new();
            asset.png_base64.hash(&mut hash);
            let key = hash.finish();
            if let Some(texture) = cache.textures.iter_mut().find(|t| t.key == key) {
                texture.used = frame;
                return Some(texture.handle.clone());
            }
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(&asset.png_base64)
                .ok()?;
            let image = image::load_from_memory_with_format(&bytes, image::ImageFormat::Png)
                .ok()?
                .to_rgba8();
            let texture = painter.ctx().load_texture(
                format!("source_{key:x}"),
                egui::ColorImage::from_rgba_unmultiplied(
                    [image.width() as usize, image.height() as usize],
                    image.as_raw(),
                ),
                egui::TextureOptions::LINEAR,
            );
            cache.textures.push(CachedTexture {
                key,
                handle: texture.clone(),
                used: frame,
            });
            Some(texture)
        })
        .collect();
    cache.prune(frame);
    painter.ctx().data_mut(|d| d.insert_temp(cache_id, cache));
    for id in page.scene_order() {
        if !visible(page, id) {
            continue;
        }
        let node = page.node(id).unwrap();
        let rect = view.rect(page.world_rect(id).unwrap());
        let clip = clip_rect(page, id, view, painter.clip_rect());
        if !clip.intersects(rect) {
            continue;
        }
        let painter = painter.with_clip_rect(clip);
        let opacity = opacity(page, id);
        let conversation = node.ui_script == [crate::actions::UiAction::Conversation];
        let mut image_background = false;
        if let Some(art) = &node.artwork
            && (editing != Some(id) || node.kind != Kind::Text || art.original_text.is_empty())
            && (node.kind != Kind::Text || art.original_text.is_empty())
        {
            if let Some(icon) = &art.vector_icon {
                use crate::document::VectorIcon;
                let ink = art
                    .meshes
                    .iter()
                    .flat_map(|m| &m.vertices)
                    .max_by_key(|v| v.color[3])
                    .map(|v| v.color)
                    .unwrap_or([210, 210, 210, 255]);
                let stroke = Stroke::new(view.zoom.max(0.5), color(ink, opacity));
                let point = |x: f32, y: f32| rect.min + rect.size() * Vec2::new(x, y);
                match icon {
                    VectorIcon::SplitPanel => {
                        painter.rect_stroke(rect, 3. * view.zoom, stroke, StrokeKind::Inside);
                    }
                    VectorIcon::SplitDivider => {
                        painter.line_segment([rect.center_top(), rect.center_bottom()], stroke);
                    }
                    VectorIcon::FolderBack => {
                        painter.add(egui::Shape::line(
                            vec![
                                point(0., 1.),
                                point(0., 0.35),
                                point(0.12, 0.),
                                point(0.42, 0.),
                                point(0.55, 0.5),
                                point(0.9, 0.5),
                                point(1., 1.),
                            ],
                            stroke,
                        ));
                    }
                    VectorIcon::FolderFront => {
                        painter.add(egui::Shape::closed_line(
                            vec![point(0., 0.), point(1., 0.), point(0.82, 1.), point(0., 1.)],
                            stroke,
                        ));
                    }
                }
                continue;
            }
            let scale = rect.size() / Vec2::new(art.size[0], art.size[1]);
            let art_clip = Rect::from_min_size(
                rect.min + Vec2::new(art.clip[0], art.clip[1]) * scale,
                Vec2::new(art.clip[2], art.clip[3]) * scale,
            );
            let painter = painter.with_clip_rect(clip.intersect(art_clip));
            for source in &art.meshes {
                let Some(Some(texture)) = textures.get(source.texture) else {
                    continue;
                };
                let mut mesh = egui::Mesh::with_texture(texture.id());
                mesh.indices = source.indices.clone();
                mesh.vertices = source
                    .vertices
                    .iter()
                    .map(|v| egui::epaint::Vertex {
                        pos: rect.min + Vec2::new(v.position[0], v.position[1]) * scale,
                        uv: Pos2::new(v.uv[0], v.uv[1]),
                        color: color(v.color, opacity),
                    })
                    .collect();
                painter.add(egui::Shape::mesh(mesh));
            }
            if (node.kind.has_text() && (node.kind != Kind::Text || art.original_text.is_empty()))
                || conversation
            {
                image_background = true;
            } else {
                continue;
            }
        }
        let fill = color(node.fill, opacity);
        let stroke = Stroke::new(node.stroke_width * view.zoom, color(node.stroke, opacity));
        if !image_background {
            match node.kind {
                Kind::Ellipse => {
                    painter.add(egui::epaint::EllipseShape {
                        center: rect.center(),
                        radius: rect.size() * 0.5,
                        fill,
                        stroke,
                    });
                }
                Kind::Line => {
                    painter.line_segment([rect.left_center(), rect.right_center()], stroke);
                }
                _ => {
                    painter.rect(
                        rect,
                        node.radius * view.zoom,
                        fill,
                        stroke,
                        StrokeKind::Inside,
                    );
                }
            }
        }
        if (node.kind.has_text() || conversation) && editing != Some(id) {
            let inset = if conversation {
                (node.padding * view.zoom)
                    .min(((rect.width().min(rect.height()) - 1.) * 0.5).max(0.))
            } else if node.kind == Kind::Text {
                0.
            } else {
                12. * view.zoom
            };
            let area = rect.shrink2(Vec2::new(inset, if conversation { inset } else { 0. }));
            let mut job = egui::text::LayoutJob::simple(
                node.text.clone(),
                font(node, view.zoom, painter.ctx()),
                color(node.text_color, opacity),
                // Captured labels have tight glyph bounds, not paragraph boxes.
                // Re-layout their actual text without wrapping it inside those old pixel bounds.
                if node.kind == Kind::Text
                    && node
                        .artwork
                        .as_ref()
                        .is_some_and(|art| node.width == art.size[0])
                {
                    f32::INFINITY
                } else {
                    area.width().max(1.)
                },
            );
            job.halign = match node.text_align {
                TextAlign::Left => egui::Align::Min,
                TextAlign::Center => egui::Align::Center,
                TextAlign::Right => egui::Align::Max,
            };
            job.wrap.max_rows = 64;
            if let Some(art) = &node.artwork
                && !art.original_text.is_empty()
                && node.width == art.size[0]
                && let Some(layout) = &art.text_layout
            {
                job.wrap.max_width = layout
                    .width
                    .map(|width| width * view.zoom)
                    .unwrap_or(f32::INFINITY);
                job.wrap.max_rows = layout.max_rows;
                job.wrap.break_anywhere = layout.break_anywhere;
                job.wrap.overflow_character = layout.overflow_character;
                if node.text_align == art.original_text_align {
                    job.halign = match layout.align {
                        TextAlign::Left => egui::Align::Min,
                        TextAlign::Center => egui::Align::Center,
                        TextAlign::Right => egui::Align::Max,
                    };
                }
            }
            let galley = painter.layout_job(job);
            let x = match node.text_align {
                TextAlign::Left => area.left(),
                TextAlign::Center => area.center().x,
                TextAlign::Right => area.right(),
            };
            let y = if node.kind == Kind::Text || conversation {
                area.top()
            } else {
                area.center().y - galley.size().y * 0.5
            };
            let text_clip = if node.kind == Kind::Text
                && node
                    .artwork
                    .as_ref()
                    .is_some_and(|art| node.width == art.size[0] && node.height == art.size[1])
            {
                let art = node.artwork.as_ref().unwrap();
                clip.intersect(Rect::from_min_size(
                    rect.min + Vec2::new(art.clip[0], art.clip[1]) * view.zoom,
                    Vec2::new(art.clip[2], art.clip[3]) * view.zoom,
                ))
            } else {
                clip.intersect(rect)
            };
            let offset = node
                .artwork
                .as_ref()
                .filter(|a| {
                    !a.original_text.is_empty()
                        && node.width == a.size[0]
                        && node.height == a.size[1]
                        && node.text_align == a.original_text_align
                })
                .map(|a| Vec2::new(a.text_origin[0], a.text_origin[1]) * view.zoom)
                .unwrap_or_default();
            painter.with_clip_rect(text_clip).galley(
                Pos2::new(x, y) + offset,
                galley,
                color(node.text_color, opacity),
            );
        }
    }
}

#[cfg(test)]
#[path = "../../tests/presentation/texture_cache.rs"]
mod texture_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::Artwork;

    #[test]
    fn captured_text_is_laid_out_again_at_each_zoom_and_monitor_scale() {
        let ctx = egui::Context::default();
        let mut page = Page::blank("DPI test");
        let mut node = Node::new(1, Kind::Text, 10., 10.);
        node.text = "Scalable label".into();
        node.width = 20.;
        node.height = 12.;
        node.font_size = 14.;
        node.artwork = Some(Artwork {
            text_layout: None,
            font_face: None,
            text_origin: [0.; 2],
            original_font: Default::default(),
            original_text_align: Default::default(),
            vector_icon: None,
            size: [20., 12.],
            meshes: vec![],
            clip: [0., 0., 20., 12.],
            original_text: node.text.clone(),
            original_color: node.text_color,
            original_font_size: 14.,
        });
        page.nodes.push(node);
        for (dpi, zoom) in [(1., 1.), (1.5, 2.), (2., 4.), (1., 0.75)] {
            let before = page.clone();
            let mut input = egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::splat(1000.))),
                ..Default::default()
            };
            input
                .viewports
                .entry(egui::ViewportId::ROOT)
                .or_default()
                .native_pixels_per_point = Some(dpi);
            let output = ctx.run(input, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    render(
                        ui.painter(),
                        &page,
                        View {
                            origin: Pos2::ZERO,
                            zoom,
                        },
                        None,
                        &[],
                    )
                });
            });
            let text = output
                .shapes
                .iter()
                .find_map(|shape| {
                    if let egui::Shape::Text(text) = &shape.shape {
                        Some(text)
                    } else {
                        None
                    }
                })
                .expect("Imported text must use a live galley, not captured glyph triangles");
            assert_eq!(text.galley.job.text, page.nodes[0].text);
            assert_eq!(text.galley.job.sections[0].format.font_id.size, 14. * zoom);
            assert_eq!(
                text.galley.rows.len(),
                1,
                "Tight imported glyph bounds must not force new line wrapping"
            );
            assert!((ctx.pixels_per_point() - dpi).abs() < 0.01);
            assert_eq!(
                page, before,
                "DPI and zoom must not modify saved design geometry"
            );
            page.nodes[0].text = "Updated label".into();
        }
    }
}
