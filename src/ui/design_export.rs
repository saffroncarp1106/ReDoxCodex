use base64::Engine;
use eframe::egui::{self, Pos2, Rect, Vec2, epaint::Primitive};
use redox_studio::document::{ArtMesh, ArtVertex, Artwork, Document, Kind, Node, TextureAsset};
use std::collections::HashMap;
#[path = "../../tests/ui/design_raster.rs"]
mod raster;

pub fn export(destination: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(destination)?;
    for id in ["codex", "claude", "gemini", "win98"] {
        if std::env::var("REDOX_EXPORT_THEME").is_ok_and(|selected| selected != id) {
            continue;
        }
        let context = egui::Context::default();
        let mut app = crate::app::ReDox::design_preview(&context, id);
        let mut textures = HashMap::new();
        let mut shapes = vec![];
        for frame_number in 0..6 {
            let output = context.run(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1100., 900.))),
                    ..Default::default()
                },
                |context| app.render_design(context),
            );
            raster::update(&mut textures, &output.textures_delta);
            if frame_number == 5 {
                let review =
                    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target/design-review");
                std::fs::create_dir_all(&review)?;
                raster::write(
                    &context,
                    output.clone(),
                    &textures,
                    &review.join(format!("{id}.png")),
                );
            }
            shapes = output.shapes;
        }
        let mut doc = Document::blank();
        doc.name = format!("ReDox {}", id);
        doc.author = "ReDox source".into();
        let mut frame = Node::new(1, Kind::Frame, 0., 0.);
        frame.name = format!("{} original 1100x900", id);
        frame.width = 1100.;
        frame.height = 900.;
        frame.fill = [24, 24, 24, 255];
        doc.pages[0].nodes.push(frame);
        let mut texture_ids = HashMap::new();
        for (id, image) in textures {
            let pixels: Vec<u8> = image
                .pixels
                .iter()
                .flat_map(|p| p.to_srgba_unmultiplied())
                .collect();
            let image =
                image::RgbaImage::from_raw(image.width() as u32, image.height() as u32, pixels)
                    .ok_or("Invalid image")?;
            let mut png = std::io::Cursor::new(vec![]);
            image.write_to(&mut png, image::ImageFormat::Png)?;
            texture_ids.insert(id, doc.textures.len());
            doc.textures.push(TextureAsset {
                png_base64: base64::engine::general_purpose::STANDARD.encode(png.into_inner()),
            });
        }
        let mut flattened = vec![];
        fn flatten(clip: Rect, shape: egui::Shape, out: &mut Vec<egui::epaint::ClippedShape>) {
            if let egui::Shape::Vec(shapes) = shape {
                for shape in shapes {
                    flatten(clip, shape, out);
                }
            } else {
                out.push(egui::epaint::ClippedShape {
                    clip_rect: clip,
                    shape,
                });
            }
        }
        for shape in shapes {
            flatten(shape.clip_rect, shape.shape, &mut flattened);
        }
        for shape in flattened {
            let text = if let egui::Shape::Text(text) = &shape.shape {
                Some((
                    text.galley.job.text.clone(),
                    text.galley
                        .job
                        .sections
                        .first()
                        .map(|s| s.format.font_id.size)
                        .unwrap_or(14.),
                    text.fallback_color.to_srgba_unmultiplied(),
                ))
            } else {
                None
            };
            let clip = shape
                .clip_rect
                .intersect(Rect::from_min_size(Pos2::ZERO, Vec2::new(1100., 900.)));
            let primitives = context.tessellate(vec![shape], 1.);
            let mut meshes = vec![];
            let mut bounds = Rect::NOTHING;
            for primitive in primitives {
                if let Primitive::Mesh(mesh) = primitive.primitive {
                    if mesh.vertices.is_empty() {
                        continue;
                    }
                    for vertex in &mesh.vertices {
                        bounds.extend_with(vertex.pos);
                    }
                    meshes.push(mesh);
                }
            }
            if meshes.is_empty() || !bounds.intersects(clip) {
                continue;
            }
            let mut node = Node::new(
                doc.next_id(),
                if text.is_some() {
                    Kind::Text
                } else {
                    Kind::Artwork
                },
                bounds.min.x,
                bounds.min.y,
            );
            node.parent = Some(1);
            node.width = bounds.width().max(1.);
            node.height = bounds.height().max(1.);
            let (label, size, color) = text.unwrap_or((String::new(), 14., [255; 4]));
            node.name = if label.is_empty() {
                format!("Artwork {}", node.id)
            } else {
                label.chars().filter(|c| !c.is_control()).take(30).collect()
            };
            if node.name.trim().is_empty() {
                node.name = format!("Text {}", node.id);
            }
            node.text = label.clone();
            node.font_size = size.clamp(8., 160.);
            node.text_color = color;
            node.artwork = Some(Artwork {
                vector_icon: None,
                size: [node.width, node.height],
                original_text: label,
                original_font_size: node.font_size,
                original_color: color,
                clip: [
                    clip.min.x - bounds.min.x,
                    clip.min.y - bounds.min.y,
                    clip.width(),
                    clip.height(),
                ],
                meshes: meshes
                    .into_iter()
                    .map(|mesh| ArtMesh {
                        texture: texture_ids[&mesh.texture_id],
                        indices: mesh.indices,
                        vertices: mesh
                            .vertices
                            .into_iter()
                            .map(|v| ArtVertex {
                                position: [v.pos.x - bounds.min.x, v.pos.y - bounds.min.y],
                                uv: [v.uv.x, v.uv.y],
                                color: v.color.to_srgba_unmultiplied(),
                            })
                            .collect(),
                    })
                    .collect(),
            });
            doc.pages[0].nodes.push(node);
        }
        doc.save(&destination.join(format!("{id}.redoxskin")))?;
        println!("Exported {id}: {} layers", doc.node_count());
    }
    Ok(())
}
