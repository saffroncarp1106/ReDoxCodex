use eframe::egui::{self, TextureId};
use std::collections::HashMap;

// Render egui's offscreen meshes for layout review without opening a desktop window.
pub fn render(
    ctx: &egui::Context,
    output: egui::FullOutput,
    textures: &HashMap<TextureId, egui::ColorImage>,
    width: u32,
    height: u32,
    cancelled: impl Fn() -> bool,
) -> Result<image::RgbaImage, String> {
    let mut remaining_work = 100_000_000u64;
    let mut pixels = image::RgbaImage::from_pixel(width, height, image::Rgba([30, 30, 33, 255]));
    for clipped in ctx.tessellate(output.shapes, output.pixels_per_point) {
        let egui::epaint::Primitive::Mesh(mesh) = clipped.primitive else {
            continue;
        };
        let Some(texture) = textures.get(&mesh.texture_id) else {
            continue;
        };
        for triangle in mesh.indices.as_chunks::<3>().0 {
            if cancelled() {
                return Err("Render cancelled".into());
            }
            let vertices =
                [triangle[0], triangle[1], triangle[2]].map(|i| mesh.vertices[i as usize]);
            let [a, b, c] = vertices.map(|v| v.pos);
            let edge = |a: egui::Pos2, b: egui::Pos2, p: egui::Pos2| {
                (b.x - a.x) * (p.y - a.y) - (b.y - a.y) * (p.x - a.x)
            };
            let area = edge(a, b, c);
            if area.abs() < 0.001 {
                continue;
            }
            let min_x =
                a.x.min(b.x)
                    .min(c.x)
                    .max(clipped.clip_rect.left())
                    .max(0.)
                    .floor() as u32;
            let min_y =
                a.y.min(b.y)
                    .min(c.y)
                    .max(clipped.clip_rect.top())
                    .max(0.)
                    .floor() as u32;
            let max_x =
                a.x.max(b.x)
                    .max(c.x)
                    .min(clipped.clip_rect.right())
                    .min(width as f32)
                    .ceil() as u32;
            let max_y =
                a.y.max(b.y)
                    .max(c.y)
                    .min(clipped.clip_rect.bottom())
                    .min(height as f32)
                    .ceil() as u32;
            let work = (max_x.saturating_sub(min_x) as u64) * (max_y.saturating_sub(min_y) as u64);
            remaining_work = remaining_work
                .checked_sub(work)
                .ok_or("Render work limit exceeded")?;
            for y in min_y..max_y {
                if cancelled() {
                    return Err("Render cancelled".into());
                }
                for x in min_x..max_x {
                    let p = egui::pos2(x as f32 + 0.5, y as f32 + 0.5);
                    let weights = [
                        edge(b, c, p) / area,
                        edge(c, a, p) / area,
                        edge(a, b, p) / area,
                    ];
                    if weights.iter().any(|w| *w < -0.00001) {
                        continue;
                    }
                    let uv = vertices
                        .iter()
                        .zip(weights)
                        .fold(egui::Vec2::ZERO, |sum, (v, w)| sum + v.uv.to_vec2() * w);
                    let tx = (uv.x * texture.size[0] as f32 - 0.5)
                        .clamp(0., texture.size[0] as f32 - 1.);
                    let ty = (uv.y * texture.size[1] as f32 - 0.5)
                        .clamp(0., texture.size[1] as f32 - 1.);
                    let sample = |x: usize, y: usize| {
                        texture.pixels[y.min(texture.size[1] - 1) * texture.size[0]
                            + x.min(texture.size[0] - 1)]
                        .to_array()
                    };
                    let samples = [
                        sample(tx as usize, ty as usize),
                        sample(tx as usize + 1, ty as usize),
                        sample(tx as usize, ty as usize + 1),
                        sample(tx as usize + 1, ty as usize + 1),
                    ];
                    let u = tx.fract();
                    let v = ty.fract();
                    let blend = [(1. - u) * (1. - v), u * (1. - v), (1. - u) * v, u * v];
                    let mut source = [0f32; 4];
                    for channel in 0..4 {
                        let texture_channel: f32 = samples
                            .iter()
                            .zip(blend)
                            .map(|(s, w)| s[channel] as f32 * w / 255.)
                            .sum();
                        let vertex_channel: f32 = vertices
                            .iter()
                            .zip(weights)
                            .map(|(v, w)| v.color.to_array()[channel] as f32 * w / 255.)
                            .sum();
                        source[channel] = texture_channel * vertex_channel;
                    }
                    let target = pixels.get_pixel_mut(x, y);
                    for channel in 0..3 {
                        target[channel] = (source[channel] * 255.
                            + target[channel] as f32 * (1. - source[3]))
                            .clamp(0., 255.) as u8;
                    }
                }
            }
        }
    }
    Ok(pixels)
}

pub fn update(textures: &mut HashMap<TextureId, egui::ColorImage>, delta: &egui::TexturesDelta) {
    for (id, delta) in &delta.set {
        let egui::ImageData::Color(image) = &delta.image;
        if let Some([x, y]) = delta.pos {
            let texture = textures.get_mut(id).unwrap();
            for row in 0..image.size[1] {
                for col in 0..image.size[0] {
                    texture[(x + col, y + row)] = image[(col, row)];
                }
            }
        } else {
            textures.insert(*id, (**image).clone());
        }
    }
    for id in &delta.free {
        textures.remove(id);
    }
}
