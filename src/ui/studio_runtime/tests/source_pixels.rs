use redox_studio::{
    presentation::render_document,
    scenes::{self, SceneId},
};

#[test]
fn windows_98_matches_the_redox_source_renderer() {
    for (scene, reference) in [
        (
            SceneId::Welcome,
            include_bytes!("reference/win98-welcome.png").as_slice(),
        ),
        (
            SceneId::About,
            include_bytes!("reference/win98-about.png").as_slice(),
        ),
    ] {
        let doc = scenes::load("win98", scene).unwrap();
        let png = render_document(&doc, 0, None, 1100, 900, || false).unwrap();
        let actual = image::load_from_memory(&png).unwrap().to_rgba8();
        let expected = image::load_from_memory(reference).unwrap().to_rgba8();
        let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target/ui-review");
        std::fs::create_dir_all(&directory).unwrap();
        actual
            .save(directory.join(format!("win98-{}-live-text.png", scene.id())))
            .unwrap();
        let mut text_mask = vec![false; 1100 * 900];
        for node in &doc.pages[0].nodes {
            if node.kind != redox_studio::document::Kind::Text {
                continue;
            }
            let bounds = doc.pages[0].world_rect(node.id).unwrap();
            for y in ((bounds[1] - 4.).floor().max(0.) as usize)
                ..((bounds[1] + bounds[3] + 4.).ceil().min(900.) as usize)
            {
                for x in ((bounds[0] - 4.).floor().max(0.) as usize)
                    ..((bounds[0] + bounds[2] + 4.).ceil().min(1100.) as usize)
                {
                    text_mask[y * 1100 + x] = true;
                }
            }
        }
        let mut error = 0u64;
        let mut geometry_error = 0u64;
        let mut geometry_channels = 0u64;
        for y in 0..900 {
            for x in 0..1100 {
                for c in 0..3 {
                    error += u64::from(actual[(x, y)][c].abs_diff(expected[(x, y)][c]));
                    if !text_mask[y as usize * 1100 + x as usize] {
                        geometry_error +=
                            u64::from(actual[(x, y)][c].abs_diff(expected[(x, y)][c]));
                        geometry_channels += 1;
                    }
                }
            }
        }
        let mean = error as f64 / (1100. * 900. * 3.);
        eprintln!(
            "Windows 98 {} mean channel error: {:.6}/255",
            scene.id(),
            mean
        );
        let geometry_mean = geometry_error as f64 / geometry_channels as f64;
        eprintln!("Geometry outside live text: {geometry_mean:.6}/255");
        // Live fonts may differ in hinting from the original captured atlas.
        assert!(mean < 2., "Source layout regression: {mean}");
        assert!(
            geometry_mean < 0.05,
            "Non-text geometry regression: {geometry_mean}"
        );
    }
}
