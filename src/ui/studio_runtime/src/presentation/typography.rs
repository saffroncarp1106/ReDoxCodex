use eframe::egui;

pub const FACES: [(&str, &str); 5] = [
    ("redox.tahoma", "tahoma.ttf"),
    ("redox.tahoma.bold", "tahomabd.ttf"),
    ("redox.segoe", "segoeui.ttf"),
    ("redox.segoe.bold", "seguisb.ttf"),
    ("redox.serif", "georgia.ttf"),
];

pub fn register(fonts: &mut egui::FontDefinitions) {
    static SOURCES: std::sync::OnceLock<Vec<(String, egui::FontData)>> = std::sync::OnceLock::new();
    let sources = SOURCES.get_or_init(|| {
        let directory = std::env::var_os("SystemRoot")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| "C:/Windows".into())
            .join("Fonts");
        FACES
            .iter()
            .filter_map(|(name, file)| {
                std::fs::read(directory.join(file))
                    .ok()
                    .map(|bytes| (name.to_string(), egui::FontData::from_owned(bytes)))
            })
            .collect()
    });
    for (name, _) in FACES {
        let mut family = fonts.families[&egui::FontFamily::Proportional].clone();
        if let Some((_, data)) = sources.iter().find(|(key, _)| key == name) {
            fonts.font_data.insert(name.into(), data.clone().into());
            family.insert(0, name.into());
        }
        fonts
            .families
            .insert(egui::FontFamily::Name(name.into()), family);
    }
}

pub fn install(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    register(&mut fonts);
    ctx.set_fonts(fonts);
}
