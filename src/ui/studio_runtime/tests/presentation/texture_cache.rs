use super::*;
use base64::Engine;

fn asset(value: u8) -> crate::document::TextureAsset {
    let image = image::RgbaImage::from_pixel(4, 4, image::Rgba([value, 100, 200, 255]));
    let mut bytes = std::io::Cursor::new(vec![]);
    image.write_to(&mut bytes, image::ImageFormat::Png).unwrap();
    crate::document::TextureAsset {
        png_base64: base64::engine::general_purpose::STANDARD.encode(bytes.into_inner()),
    }
}

fn draw(ctx: &egui::Context, assets: &[crate::document::TextureAsset]) {
    let _ = ctx.run(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::splat(400.))),
            ..Default::default()
        },
        |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                render(
                    ui.painter(),
                    &Page::blank("Cache test"),
                    View {
                        origin: Pos2::ZERO,
                        zoom: 1.,
                    },
                    None,
                    assets,
                )
            });
        },
    );
}

#[test]
fn repeated_asset_swaps_bound_the_texture_cache_and_reuse_current_images() {
    let ctx = egui::Context::default();
    let key = egui::Id::new(("skin_textures", egui::ViewportId::ROOT));
    for value in 0..50 {
        draw(&ctx, &[asset(value)]);
    }
    let cache = ctx.data(|d| d.get_temp::<TextureCache>(key)).unwrap();
    assert!(cache.textures.len() <= MAX_CACHED_TEXTURES);
    let current = cache.textures.last().unwrap().handle.id();
    draw(&ctx, &[asset(49)]);
    let cache = ctx.data(|d| d.get_temp::<TextureCache>(key)).unwrap();
    assert_eq!(cache.textures.last().unwrap().handle.id(), current);
}

#[test]
fn pruning_preserves_every_texture_referenced_in_the_current_frame() {
    let ctx = egui::Context::default();
    let assets: Vec<_> = (0..40).map(asset).collect();
    draw(&ctx, &assets);
    let key = egui::Id::new(("skin_textures", egui::ViewportId::ROOT));
    let cache = ctx.data(|d| d.get_temp::<TextureCache>(key)).unwrap();
    assert_eq!(cache.textures.len(), 40);
    let current = cache.textures.last().unwrap().handle.id();
    draw(&ctx, &assets[39..]);
    let cache = ctx.data(|d| d.get_temp::<TextureCache>(key)).unwrap();
    assert!(cache.textures.len() <= MAX_CACHED_TEXTURES);
    assert!(cache.textures.iter().any(|t| t.handle.id() == current));
}
