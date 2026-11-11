pub mod raster;
pub mod scene;
pub mod typography;

use crate::{
    document::{Document, Kind},
    editor::union,
};
use eframe::egui::{self, Pos2, Rect, Vec2};
use std::collections::HashMap;

pub fn render_document(
    document: &Document,
    page_index: usize,
    frame_id: Option<u32>,
    width: u32,
    height: u32,
    cancelled: impl Fn() -> bool,
) -> Result<Vec<u8>, String> {
    if width == 0
        || height == 0
        || width > 2048
        || height > 2048
        || width as u64 * height as u64 > 4_000_000
    {
        return Err("Render exceeds its pixel limits".into());
    }
    let mut page = document
        .pages
        .get(page_index)
        .ok_or("Page is missing")?
        .clone();
    if let Some(id) = frame_id {
        let node = page.node(id).ok_or("Frame is missing")?;
        if node.kind != Kind::Frame {
            return Err("Select a frame to render".into());
        }
        let bounds = page.world_rect(id).ok_or("Frame parent is missing")?;
        let ids: Vec<_> = page
            .nodes
            .iter()
            .filter(|n| n.id == id || page.ancestors(n.id).contains(&id))
            .map(|n| n.id)
            .collect();
        page.nodes.retain(|n| ids.contains(&n.id));
        let root = page.node_mut(id).unwrap();
        root.parent = None;
        root.x = bounds[0];
        root.y = bounds[1];
    }
    let bounds = union(
        page.nodes
            .iter()
            .filter(|n| n.parent.is_none() && n.visible)
            .filter_map(|n| page.world_rect(n.id)),
    )
    .unwrap_or([0., 0., 960., 640.]);
    let text_work: usize = page
        .nodes
        .iter()
        .filter(|n| {
            n.visible
                && (n.kind.has_text() || n.ui_script == [crate::actions::UiAction::Conversation])
        })
        .map(|n| n.text.chars().count())
        .sum();
    if text_work > 20_000 {
        return Err("Render exceeds 20000 text characters".into());
    }
    let zoom = (width as f32 / bounds[2])
        .min(height as f32 / bounds[3])
        .min(4.);
    let origin = Pos2::new(
        (width as f32 - bounds[2] * zoom) * 0.5 - bounds[0] * zoom,
        (height as f32 - bounds[3] * zoom) * 0.5 - bounds[1] * zoom,
    );
    let context = egui::Context::default();
    typography::install(&context);
    let output = context.run(
        egui::RawInput {
            max_texture_side: Some(4096),
            screen_rect: Some(Rect::from_min_size(
                Pos2::ZERO,
                Vec2::new(width as f32, height as f32),
            )),
            ..Default::default()
        },
        |ctx| {
            egui::CentralPanel::default()
                .frame(egui::Frame::NONE)
                .show(ctx, |ui| {
                    ui.painter().rect_filled(
                        ui.max_rect(),
                        0,
                        egui::Color32::from_rgb(
                            page.background[0],
                            page.background[1],
                            page.background[2],
                        ),
                    );
                    scene::render(
                        ui.painter(),
                        &page,
                        scene::View { origin, zoom },
                        None,
                        &document.textures,
                    );
                });
        },
    );
    let mut textures = HashMap::new();
    raster::update(&mut textures, &output.textures_delta);
    let image = raster::render(&context, output, &textures, width, height, cancelled)?;
    let mut bytes = std::io::Cursor::new(Vec::new());
    image
        .write_to(&mut bytes, image::ImageFormat::Png)
        .map_err(|e| e.to_string())?;
    let bytes = bytes.into_inner();
    if bytes.len() > 8 * 1024 * 1024 {
        return Err("Render exceeds 8 MiB".into());
    }
    Ok(bytes)
}

pub mod interactive;
