use super::*;

#[test]
fn generated_windows_paths_keep_the_separator_before_dot_folders() {
    let source = r"![Image](<C:\Users\someone\.codex\generated_images\thread\dog.png>)";
    let media = references(source);
    assert_eq!(media.len(), 1);
    assert_eq!(
        media[0].destination,
        "C:/Users/someone/.codex/generated_images/thread/dog.png"
    );
}

#[test]
fn markdown_media_preserves_surrounding_text_and_ignores_code() {
    let source = "Before ![dog](<C:/Project files/dog.png>) after.\n\n[Animation](clip.gif)\n\n[Video](movie.mp4)\n\n`![literal](ignore.png)`\n\n```md\n![example](also-ignore.jpg)\n```";
    let media = references(source);
    assert_eq!(media.len(), 3);
    assert_eq!(media[0].destination, "C:/Project files/dog.png");
    assert_eq!(
        &source[media[0].range.clone()],
        "![dog](<C:/Project files/dog.png>)"
    );
    assert!(media[2].video);
    assert!(!media[1].video);
}

#[test]
fn file_urls_and_workspace_relative_paths_resolve_to_the_same_file() {
    let root = std::env::current_dir().unwrap().join("Project files");
    let (relative, open_url) = locations("dog.png", Some(&root)).unwrap();
    let (absolute, _) = locations(&open_url, None).unwrap();
    assert_eq!(relative, absolute);
    assert!(open_url.contains("Project%20files"));
    assert_eq!(
        locations("https://example.com/dog.gif", None).unwrap().0,
        "https://example.com/dog.gif"
    );
}

#[test]
fn local_png_jpeg_webp_and_animated_gif_decode_without_a_model_call() {
    use image::ImageEncoder;
    let pixels = image::RgbaImage::from_pixel(8, 8, image::Rgba([40, 140, 220, 255]));
    for format in [
        image::ImageFormat::Png,
        image::ImageFormat::Jpeg,
        image::ImageFormat::WebP,
    ] {
        let mut bytes = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(pixels.clone())
            .to_rgb8()
            .write_to(&mut bytes, format)
            .unwrap();
        assert_eq!(image::load_from_memory(bytes.get_ref()).unwrap().width(), 8);
    }
    let mut gif = Vec::new();
    {
        let mut encoder = image::codecs::gif::GifEncoder::new(&mut gif);
        for color in [[220, 100, 40, 255], [40, 140, 220, 255]] {
            encoder
                .encode_frame(image::Frame::from_parts(
                    image::RgbaImage::from_pixel(8, 8, image::Rgba(color)),
                    0,
                    0,
                    image::Delay::from_numer_denom_ms(500, 1),
                ))
                .unwrap();
        }
    }
    use image::AnimationDecoder;
    let frames = image::codecs::gif::GifDecoder::new(std::io::Cursor::new(&gif))
        .unwrap()
        .into_frames()
        .collect_frames()
        .unwrap();
    assert_eq!(frames.len(), 2);
    assert_ne!(frames[0].buffer(), frames[1].buffer());
    let context = egui::Context::default();
    egui_extras::install_image_loaders(&context);
    context.include_bytes("bytes://fixture.gif", gif);
    let frame0 = context
        .try_load_image("bytes://fixture.gif#0", egui::load::SizeHint::default())
        .unwrap();
    let frame1 = context
        .try_load_image("bytes://fixture.gif#1", egui::load::SizeHint::default())
        .unwrap();
    match (frame0, frame1) {
        (egui::load::ImagePoll::Ready { image: a }, egui::load::ImagePoll::Ready { image: b }) => {
            assert_ne!(a.pixels, b.pixels)
        }
        _ => panic!("GIF animation frames did not load"),
    }
    let mut png = Vec::new();
    image::codecs::png::PngEncoder::new(&mut png)
        .write_image(pixels.as_raw(), 8, 8, image::ExtendedColorType::Rgba8)
        .unwrap();
    context.include_bytes("bytes://fixture.png", png);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    loop {
        let image = context
            .try_load_image("bytes://fixture.png", egui::load::SizeHint::default())
            .unwrap();
        if matches!(image, egui::load::ImagePoll::Ready { .. }) {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "PNG loader did not finish"
        );
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
}
