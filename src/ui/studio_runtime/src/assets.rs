use crate::document::{ArtMesh, ArtVertex, Artwork, Document, TextureAsset};
use base64::Engine;

/// Replace one layer's artwork transactionally without changing its ID or calls.
pub fn replace_png(
    document: &mut Document,
    page: usize,
    id: u32,
    bytes: &[u8],
) -> Result<(), String> {
    if bytes.len() > 4 * 1024 * 1024 {
        return Err("Choose a PNG under 4 MiB".into());
    }
    let reader =
        image::ImageReader::with_format(std::io::Cursor::new(bytes), image::ImageFormat::Png);
    let (w, h) = reader.into_dimensions().map_err(|e| e.to_string())?;
    if w == 0 || h == 0 || w > 4096 || h > 4096 || u64::from(w) * u64::from(h) > 4 * 1024 * 1024 {
        return Err("PNG exceeds supported dimensions".into());
    }
    // Decode before admitting a new asset, so corrupt pixel data cannot be saved.
    image::load_from_memory_with_format(bytes, image::ImageFormat::Png)
        .map_err(|e| e.to_string())?;
    let mut staged = document.clone();
    let index = staged.textures.len();
    let node = staged
        .pages
        .get_mut(page)
        .and_then(|p| p.node_mut(id))
        .ok_or("Layer not found")?;
    let font_face = node
        .artwork
        .as_ref()
        .filter(|art| art.original_font == node.font)
        .and_then(|art| art.font_face.clone());
    node.artwork = Some(Artwork {
        text_layout: None,
        font_face,
        text_origin: [0.; 2],
        original_font: node.font,
        original_text_align: Default::default(),
        vector_icon: None,
        size: [w as f32, h as f32],
        clip: [0., 0., w as f32, h as f32],
        original_text: String::new(),
        original_color: [255; 4],
        original_font_size: 14.,
        meshes: vec![ArtMesh {
            texture: index,
            vertices: [
                ([0., 0.], [0., 0.]),
                ([w as f32, 0.], [1., 0.]),
                ([w as f32, h as f32], [1., 1.]),
                ([0., h as f32], [0., 1.]),
            ]
            .into_iter()
            .map(|(position, uv)| ArtVertex {
                position,
                uv,
                color: [255; 4],
            })
            .collect(),
            indices: vec![0, 1, 2, 0, 2, 3],
        }],
    });
    staged.textures.push(TextureAsset {
        png_base64: base64::engine::general_purpose::STANDARD.encode(bytes),
    });
    compact_textures(&mut staged);
    staged.validate()?;
    *document = staged;
    Ok(())
}

/// Remove captured drawing data while preserving layout, text and behavior.
pub fn detach(document: &mut Document, page: usize, id: u32) -> Result<(), String> {
    let mut staged = document.clone();
    let node = staged
        .pages
        .get_mut(page)
        .and_then(|p| p.node_mut(id))
        .ok_or("Layer not found")?;
    let art = node
        .artwork
        .take()
        .ok_or("This layer already uses editable styling")?;
    if node.kind == crate::document::Kind::Artwork {
        node.kind = crate::document::Kind::Rectangle;
        node.fill = art
            .meshes
            .iter()
            .flat_map(|m| &m.vertices)
            .max_by_key(|v| v.color[3])
            .map(|v| v.color)
            .unwrap_or([217, 222, 234, 255]);
        node.stroke_width = 0.;
    }
    compact_textures(&mut staged);
    staged.validate()?;
    *document = staged;
    Ok(())
}

fn compact_textures(document: &mut Document) {
    let used: std::collections::BTreeSet<_> = document
        .pages
        .iter()
        .flat_map(|p| &p.nodes)
        .filter_map(|n| n.artwork.as_ref())
        .flat_map(|a| &a.meshes)
        .map(|m| m.texture)
        .collect();
    let mapping: std::collections::BTreeMap<_, _> = used
        .iter()
        .enumerate()
        .map(|(new, old)| (*old, new))
        .collect();
    document.textures = used.iter().map(|i| document.textures[*i].clone()).collect();
    for mesh in document
        .pages
        .iter_mut()
        .flat_map(|p| &mut p.nodes)
        .filter_map(|n| n.artwork.as_mut())
        .flat_map(|a| &mut a.meshes)
    {
        mesh.texture = mapping[&mesh.texture];
    }
}
