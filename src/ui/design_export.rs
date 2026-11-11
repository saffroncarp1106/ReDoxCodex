use base64::Engine;
use eframe::egui::{self, Pos2, Rect, Vec2, epaint::Primitive};
use redox_studio::document::{ArtMesh, ArtVertex, Artwork, Document, Kind, Node, TextureAsset};
use std::collections::HashMap;
#[path = "../../tests/ui/design_raster.rs"]
mod raster;

pub fn export(destination: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(destination)?;
    for id in redox_studio::scenes::THEMES {
        if std::env::var("REDOX_EXPORT_THEME").is_ok_and(|selected| selected != id) {
            continue;
        }
        for scene in redox_studio::scenes::SceneId::ALL {
            if std::env::var("REDOX_EXPORT_SCENE").is_ok_and(|selected| selected != scene.id()) {
                continue;
            }
            let scene_key = format!("{}-{}", id, scene.id());
            let context = egui::Context::default();
            let mut app = crate::app::ReDox::design_preview(&context, id);
            app.prepare_design_scene(scene);
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
                if frame_number == 5 && std::env::var_os("REDOX_EXPORT_REVIEW").is_some() {
                    let review = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                        .join("target/design-review");
                    std::fs::create_dir_all(&review)?;
                    raster::write(
                        &context,
                        output.clone(),
                        &textures,
                        &review.join(format!("{scene_key}.png")),
                    );
                }
                shapes = output.shapes;
            }
            let mut doc = Document::blank();
            doc.name = format!("ReDox {} {}", id, scene.label());
            doc.source = Some(redox_studio::scenes::Source {
                theme: id.into(),
                scene,
            });
            doc.pages[0].name = scene.label().into();
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
                let text =
                    if let egui::Shape::Text(text) = &shape.shape {
                        Some((
                            text.galley.job.text.clone(),
                            text.galley
                                .job
                                .sections
                                .first()
                                .map(|s| s.format.font_id.size)
                                .unwrap_or(14.),
                            text.override_text_color
                                .or_else(|| {
                                    text.galley
                                        .job
                                        .sections
                                        .first()
                                        .map(|section| section.format.color)
                                        .filter(|color| *color != egui::Color32::PLACEHOLDER)
                                })
                                .unwrap_or(text.fallback_color)
                                .to_srgba_unmultiplied(),
                            if text.galley.job.sections.first().is_some_and(|s| {
                                s.format.font_id.family == egui::FontFamily::Monospace
                            }) {
                                redox_studio::document::Font::Mono
                            } else {
                                redox_studio::document::Font::Sans
                            },
                        ))
                    } else {
                        None
                    };
                let clip = shape
                    .clip_rect
                    .intersect(Rect::from_min_size(Pos2::ZERO, Vec2::new(1100., 900.)));
                let primitives = context.tessellate(vec![shape.clone()], 1.);
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
                let (label, size, color, font) =
                    text.unwrap_or((String::new(), 14., [255; 4], Default::default()));
                node.name = if label.is_empty() {
                    format!("Artwork {}", node.id)
                } else {
                    label.chars().filter(|c| !c.is_control()).take(30).collect()
                };
                if node.name.trim().is_empty() {
                    node.name = format!("Text {}", node.id);
                }
                node.ui_script = redox_studio::actions::UiAction::from_label(&label)
                    .into_iter()
                    .collect();
                node.font = font;
                node.text = label.clone();
                node.font_size = size.clamp(8., 160.);
                node.text_color = color;
                node.artwork = Some(Artwork {
                    text_layout: if let egui::Shape::Text(text) = &shape.shape {
                        let wrap = &text.galley.job.wrap;
                        Some(redox_studio::document::SourceTextLayout {
                            width: wrap
                                .max_width
                                .is_finite()
                                .then(|| wrap.max_width.clamp(1., 16000.)),
                            max_rows: wrap.max_rows.clamp(1, 64),
                            break_anywhere: wrap.break_anywhere,
                            overflow_character: wrap.overflow_character,
                            align: match text.galley.job.halign {
                                egui::Align::Min => redox_studio::document::TextAlign::Left,
                                egui::Align::Center => redox_studio::document::TextAlign::Center,
                                egui::Align::Max => redox_studio::document::TextAlign::Right,
                            },
                        })
                    } else {
                        None
                    },
                    font_face: if let egui::Shape::Text(text) = &shape.shape {
                        let family = text
                            .galley
                            .job
                            .sections
                            .first()
                            .map(|s| &s.format.font_id.family);
                        match family {
                            Some(egui::FontFamily::Monospace) => None,
                            Some(egui::FontFamily::Name(name)) if name.as_ref() == "serif" => {
                                Some("redox.serif".into())
                            }
                            Some(egui::FontFamily::Name(name)) if name.as_ref() == "semibold" => {
                                Some(
                                    if id == "win98" {
                                        "redox.tahoma.bold"
                                    } else {
                                        "redox.segoe.bold"
                                    }
                                    .into(),
                                )
                            }
                            _ => Some(
                                if id == "win98" {
                                    "redox.tahoma"
                                } else {
                                    "redox.segoe"
                                }
                                .into(),
                            ),
                        }
                    } else {
                        None
                    },
                    text_origin: if let egui::Shape::Text(text) = &shape.shape {
                        [text.pos.x - bounds.min.x, text.pos.y - bounds.min.y]
                    } else {
                        [0.; 2]
                    },
                    original_font: node.font,
                    original_text_align: node.text_align,
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
                                    uv: [v.uv.x.clamp(0., 1.), v.uv.y.clamp(0., 1.)],
                                    color: v.color.to_srgba_unmultiplied(),
                                })
                                .collect(),
                        })
                        .collect(),
                });
                doc.pages[0].nodes.push(node);
            }
            for (rect, action) in recorded(&context) {
                let mut node = Node::new(doc.next_id(), Kind::Button, rect.min.x, rect.min.y);
                node.parent = Some(1);
                node.name = format!("Control {}", action.id());
                node.width = rect.width().max(1.);
                node.height = rect.height().max(1.);
                node.fill = [0; 4];
                node.stroke_width = 0.;
                node.text.clear();
                node.ui_script = vec![action];
                doc.pages[0].nodes.push(node);
            }
            compact_textures(&mut doc)?;
            for node in &doc.pages[0].nodes {
                if let Some(art) = &node.artwork {
                    for vertex in art.meshes.iter().flat_map(|m| &m.vertices) {
                        if vertex
                            .position
                            .iter()
                            .any(|v| !v.is_finite() || v.abs() > 32000.)
                            || vertex
                                .uv
                                .iter()
                                .any(|v| !v.is_finite() || !(0.0..=1.0).contains(v))
                        {
                            return Err(format!(
                                "Invalid vertex in {} {}: {:?}",
                                scene_key, node.name, vertex
                            )
                            .into());
                        }
                    }
                }
            }
            let folder = destination.join("scenes");
            std::fs::create_dir_all(&folder)?;
            doc.save(&folder.join(format!("{scene_key}.redoxskin")))?;
            if scene == redox_studio::scenes::SceneId::Welcome {
                let path = destination.join(format!("{id}.redoxskin"));
                if path.is_file() {
                    let archive = destination.join("../../../ReCreation/temp/pre-live-scenes");
                    std::fs::create_dir_all(&archive)?;
                    let archived = archive.join(format!("{id}.redoxskin"));
                    if !archived.exists() {
                        std::fs::copy(&path, archived)?;
                    }
                }
                doc.save(&path)?;
            }
            println!("Exported {scene_key}: {} layers", doc.node_count());
        }
    }
    Ok(())
}

pub fn record(ctx: &egui::Context, rect: Rect, action: redox_studio::actions::UiAction) {
    ctx.data_mut(|d| {
        let entries = d.get_temp_mut_or_default::<Vec<(Rect, redox_studio::actions::UiAction)>>(
            egui::Id::new("export_controls"),
        );
        entries.push((rect, action));
    });
}
fn recorded(ctx: &egui::Context) -> Vec<(Rect, redox_studio::actions::UiAction)> {
    ctx.data_mut(|d| d.get_temp(egui::Id::new("export_controls")))
        .unwrap_or_default()
}

fn compact_textures(doc: &mut Document) -> Result<(), Box<dyn std::error::Error>> {
    let used: std::collections::BTreeSet<_> = doc
        .pages
        .iter()
        .flat_map(|p| &p.nodes)
        .filter_map(|n| n.artwork.as_ref())
        .flat_map(|a| &a.meshes)
        .map(|m| m.texture)
        .collect();
    let mapping: HashMap<_, _> = used
        .iter()
        .enumerate()
        .map(|(new, old)| (*old, new))
        .collect();
    doc.textures = used.iter().map(|i| doc.textures[*i].clone()).collect();
    for mesh in doc
        .pages
        .iter_mut()
        .flat_map(|p| &mut p.nodes)
        .filter_map(|n| n.artwork.as_mut())
        .flat_map(|a| &mut a.meshes)
    {
        mesh.texture = mapping[&mesh.texture];
    }
    if doc.textures.len() <= 8 {
        return Ok(());
    }
    let images = doc
        .textures
        .iter()
        .map(|asset| {
            let bytes = base64::engine::general_purpose::STANDARD.decode(&asset.png_base64)?;
            Ok(image::load_from_memory(&bytes)?.to_rgba8())
        })
        .collect::<Result<Vec<_>, Box<dyn std::error::Error>>>()?;
    let mut best = None;
    for width in [512u32, 1024, 2048, 4096] {
        let (mut x, mut y, mut row) = (0, 0, 0);
        let mut positions = vec![];
        let mut fits = true;
        for image in &images {
            let (w, h) = (image.width() + 2, image.height() + 2);
            if w > width {
                fits = false;
                break;
            }
            if x + w > width {
                x = 0;
                y += row;
                row = 0;
            }
            positions.push((x + 1, y + 1));
            x += w;
            row = row.max(h);
        }
        let height = y + row;
        let area = u64::from(width) * u64::from(height);
        if fits
            && height <= 4096
            && area <= 4 * 1024 * 1024
            && best
                .as_ref()
                .is_none_or(|(old_area, _, _, _)| area < *old_area)
        {
            best = Some((area, width, height, positions));
        }
    }
    let (_, width, height, positions) =
        best.ok_or("Source images cannot fit within the skin texture budget")?;
    let mut atlas = image::RgbaImage::new(width, height);
    for (image, &(x, y)) in images.iter().zip(&positions) {
        for dy in 0..image.height() + 2 {
            for dx in 0..image.width() + 2 {
                let pixel = image.get_pixel(
                    dx.saturating_sub(1).min(image.width() - 1),
                    dy.saturating_sub(1).min(image.height() - 1),
                );
                atlas.put_pixel(x - 1 + dx, y - 1 + dy, *pixel);
            }
        }
    }
    for mesh in doc
        .pages
        .iter_mut()
        .flat_map(|p| &mut p.nodes)
        .filter_map(|n| n.artwork.as_mut())
        .flat_map(|a| &mut a.meshes)
    {
        let image = &images[mesh.texture];
        let (x, y) = positions[mesh.texture];
        for vertex in &mut mesh.vertices {
            vertex.uv = [
                (x as f32 + vertex.uv[0] * image.width() as f32) / width as f32,
                (y as f32 + vertex.uv[1] * image.height() as f32) / height as f32,
            ];
        }
        mesh.texture = 0;
    }
    let mut png = std::io::Cursor::new(vec![]);
    atlas.write_to(&mut png, image::ImageFormat::Png)?;
    doc.textures = vec![TextureAsset {
        png_base64: base64::engine::general_purpose::STANDARD.encode(png.into_inner()),
    }];
    Ok(())
}
