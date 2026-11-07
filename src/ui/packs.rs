use std::path::{Component, Path, PathBuf};

use eframe::egui::Color32;
use serde::{Deserialize, Serialize};

use super::theme;

#[derive(Clone, Default, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Colors {
    pub background: Option<String>,
    pub sidebar: Option<String>,
    pub rail: Option<String>,
    pub surface: Option<String>,
    pub input_background: Option<String>,
    pub text: Option<String>,
    pub menu_text: Option<String>,
    pub muted: Option<String>,
    pub border: Option<String>,
    pub accent: Option<String>,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub name: String,
    pub layout: String,
    pub heading: String,
    pub placeholder: String,
    pub palette: Colors,
    pub palette_guide: std::collections::BTreeMap<String, String>,
    pub glow: String,
    pub glow_strength: f32,
    pub sidebar_default: bool,
    pub background_image: Option<String>,
    pub background_opacity: f32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            name: "Custom".into(),
            layout: "codex".into(),
            heading: "What can I help with?".into(),
            placeholder: "Ask anything".into(),
            palette: Colors::default(),
            palette_guide: std::collections::BTreeMap::new(),
            glow: "#223C9D".into(),
            glow_strength: 0.6,
            sidebar_default: true,
            background_image: None,
            background_opacity: 1.0,
        }
    }
}

#[derive(Clone)]
pub struct Pack {
    pub id: String,
    pub config: Config,
    pub path: PathBuf,
}

pub struct Catalog {
    pub packs: Vec<Pack>,
    pub errors: Vec<String>,
}

pub fn root() -> PathBuf {
    if let Some(path) = std::env::var_os("REDOX_UI_FOLDER") {
        return path.into();
    }
    if let Ok(executable) = std::env::current_exe()
        && let Some(directory) = executable.parent()
    {
        let portable = directory.join("ui/layouts");
        if portable.is_dir() {
            return portable;
        }
        for ancestor in directory.ancestors() {
            let path = ancestor.join("src/ui/layouts");
            if path.is_dir() {
                return path;
            }
        }
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/ui/layouts")
}

pub fn color(value: &str) -> Result<Color32, String> {
    let hex = value.strip_prefix('#').unwrap_or(value);
    if hex.len() != 6 {
        return Err(format!("Invalid hex color: {value}"));
    }
    let rgb = u32::from_str_radix(hex, 16).map_err(|_| format!("Invalid hex color: {value}"))?;
    Ok(Color32::from_rgb(
        (rgb >> 16) as u8,
        (rgb >> 8) as u8,
        rgb as u8,
    ))
}

impl Config {
    pub fn validate(&self, path: &Path) -> Result<(), String> {
        if self.name.trim().is_empty()
            || self.name.len() > 80
            || self.heading.len() > 300
            || self.placeholder.len() > 200
        {
            return Err("UI names and labels must be nonempty and reasonably short.".into());
        }
        if !self.glow_strength.is_finite() || !(0.0..=1.0).contains(&self.glow_strength) {
            return Err("glow_strength must be between 0 and 1.".into());
        }
        self.colors()?;
        color(&self.glow)?;
        if !self.background_opacity.is_finite() || !(0.0..=1.0).contains(&self.background_opacity) {
            return Err("Background opacity must be between 0 and 1.".into());
        }
        if let Some(asset) = &self.background_image {
            load_background(&asset_path(path, asset)?)?;
        }
        Ok(())
    }

    pub fn colors(&self) -> Result<theme::Palette, String> {
        let mut palette = theme::Palette::default();
        for (value, target) in [
            (&self.palette.background, &mut palette.background),
            (&self.palette.sidebar, &mut palette.sidebar),
            (&self.palette.rail, &mut palette.rail),
            (&self.palette.surface, &mut palette.surface),
            (&self.palette.text, &mut palette.text),
            (&self.palette.menu_text, &mut palette.menu_text),
            (&self.palette.muted, &mut palette.muted),
            (&self.palette.border, &mut palette.border),
            (&self.palette.accent, &mut palette.blue),
        ] {
            if let Some(value) = value {
                *target = color(value)?;
            }
        }
        palette.input_background = self
            .palette
            .input_background
            .as_deref()
            .map(color)
            .transpose()?
            .unwrap_or(palette.surface);
        Ok(palette)
    }
}

pub fn asset_path(config: &Path, asset: &str) -> Result<PathBuf, String> {
    let relative = Path::new(asset);
    if relative
        .components()
        .any(|part| !matches!(part, Component::Normal(_)))
    {
        return Err("UI assets must be relative files inside the pack folder.".into());
    }
    let parent = config
        .parent()
        .ok_or("Missing UI pack directory.")?
        .canonicalize()
        .map_err(|error| error.to_string())?;
    let path = parent
        .join(relative)
        .canonicalize()
        .map_err(|_| format!("Missing UI asset: {asset}"))?;
    if !path.starts_with(parent) {
        return Err("UI asset is outside its pack folder.".into());
    }
    Ok(path)
}

pub fn read(path: &Path) -> Result<Config, String> {
    let bytes = std::fs::read(path).map_err(|error| error.to_string())?;
    if bytes.len() > 64 * 1024 {
        return Err("UI config exceeds 64 KiB.".into());
    }
    let config: Config = serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
    config.validate(path)?;
    Ok(config)
}

pub fn discover(directory: &Path) -> Catalog {
    let mut packs = Vec::new();
    let mut errors = Vec::new();
    for renderer in super::renderers::RENDERERS {
        let id = renderer.id;
        packs.push(Pack {
            id: id.into(),
            config: serde_json::from_str(renderer.config).expect("Built-in UI config"),
            path: directory.join(format!("{id}.json")),
        });
    }
    if let Ok(entries) = std::fs::read_dir(directory) {
        let mut paths: Vec<_> = entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .collect();
        paths.sort();
        for path in paths {
            let config_path = if path.is_dir() {
                path.join("ui.json")
            } else {
                path.clone()
            };
            if config_path.extension().and_then(|ext| ext.to_str()) != Some("json")
                || !config_path.is_file()
            {
                continue;
            }
            let id = path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            match read(&config_path) {
                Ok(config) => {
                    if let Some(pack) = packs.iter_mut().find(|pack| pack.id == id) {
                        pack.config = config;
                        pack.path = config_path;
                    } else {
                        packs.push(Pack {
                            id,
                            config,
                            path: config_path,
                        });
                    }
                }
                Err(error) => errors.push(format!("{}: {error}", config_path.display())),
            }
        }
    }
    Catalog { packs, errors }
}

pub fn install(source: &Path, directory: &Path) -> Result<String, String> {
    let source = if source.is_dir() {
        source.join("ui.json")
    } else {
        source.to_owned()
    };
    let config = read(&source)?;
    let image = config
        .background_image
        .as_deref()
        .map(|asset| asset_path(&source, asset))
        .transpose()?;
    save(&config, image.as_deref(), directory)
}

pub fn save(config: &Config, image: Option<&Path>, directory: &Path) -> Result<String, String> {
    let mut config = config.clone();
    config.background_image = None;
    config.validate(&directory.join("ui.json"))?;
    let background = image.map(load_background).transpose()?;
    let background = background
        .map(|image| {
            let mut bytes = std::io::Cursor::new(Vec::new());
            image
                .write_to(&mut bytes, image::ImageFormat::Png)
                .map_err(|error| error.to_string())?;
            let bytes = bytes.into_inner();
            if bytes.len() > 8 * 1024 * 1024 {
                return Err(
                    "This picture exceeds 8 MiB after conversion. Choose a smaller picture."
                        .to_owned(),
                );
            }
            Ok(bytes)
        })
        .transpose()?;
    if background.is_some() {
        config.background_image = Some("background.png".into());
    }
    let config_bytes = serde_json::to_vec_pretty(&config).map_err(|error| error.to_string())?;
    if config_bytes.len() > 64 * 1024 {
        return Err("UI config exceeds 64 KiB.".into());
    }
    std::fs::create_dir_all(directory).map_err(|error| error.to_string())?;
    let slug: String = config
        .name
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    let slug = if slug.trim_matches('-').is_empty() {
        "custom"
    } else {
        slug.trim_matches('-')
    };
    let mut id = slug.to_owned();
    let mut suffix = 1;
    while directory.join(&id).exists()
        || directory.join(format!("{id}.json")).exists()
        || super::renderers::RENDERERS
            .iter()
            .any(|renderer| renderer.id == id)
    {
        id = format!("{slug}-{suffix}");
        suffix += 1;
    }
    let destination = directory.join(&id);
    std::fs::create_dir(&destination).map_err(|error| error.to_string())?;
    if let Some(background) = background {
        std::fs::write(destination.join("background.png"), background)
            .map_err(|error| error.to_string())?;
    }
    std::fs::write(destination.join("ui.json"), config_bytes).map_err(|error| error.to_string())?;
    Ok(id)
}

pub fn load_background(path: &Path) -> Result<image::DynamicImage, String> {
    let file = std::fs::File::open(path).map_err(|error| error.to_string())?;
    if file.metadata().map_err(|error| error.to_string())?.len() > 8 * 1024 * 1024 {
        return Err("Choose a picture smaller than 8 MiB.".into());
    }
    let reader = image::ImageReader::new(std::io::BufReader::new(file))
        .with_guessed_format()
        .map_err(|error| error.to_string())?;
    if !matches!(
        reader.format(),
        Some(
            image::ImageFormat::Png
                | image::ImageFormat::Jpeg
                | image::ImageFormat::WebP
                | image::ImageFormat::Gif
        )
    ) {
        return Err("Choose a PNG, JPEG, WebP, or GIF picture.".into());
    }
    let (width, height) = reader
        .into_dimensions()
        .map_err(|error| error.to_string())?;
    if width == 0 || height == 0 || width > 4096 || height > 4096 {
        return Err("Choose a picture up to 4096 x 4096 pixels.".into());
    }
    image::open(path).map_err(|error| error.to_string())
}

#[cfg(test)]
#[path = "../../tests/ui/packs.rs"]
mod tests;
