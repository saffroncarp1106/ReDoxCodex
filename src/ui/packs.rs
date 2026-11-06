use std::path::{Component, Path, PathBuf};

use eframe::egui::Color32;
use serde::{Deserialize, Serialize};

use super::theme;

#[derive(Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Layout {
    #[default]
    Codex,
    Gemini,
}

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
    pub layout: Layout,
    pub heading: String,
    pub placeholder: String,
    pub palette: Colors,
    pub palette_guide: std::collections::BTreeMap<String, String>,
    pub glow: String,
    pub glow_strength: f32,
    pub sidebar_default: bool,
    pub background_image: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            name: "Custom".into(),
            layout: Layout::Codex,
            heading: "What can I help with?".into(),
            placeholder: "Ask anything".into(),
            palette: Colors::default(),
            palette_guide: std::collections::BTreeMap::new(),
            glow: "#223C9D".into(),
            glow_strength: 0.6,
            sidebar_default: true,
            background_image: None,
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
    if let Some(path) = std::env::var_os("CODEX_RUST_UI_FOLDER") {
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
        if let Some(asset) = &self.background_image {
            let asset = asset_path(path, asset)?;
            if asset.extension().and_then(|ext| ext.to_str()) != Some("png") {
                return Err("UI background images must be PNG files.".into());
            }
            let bytes = std::fs::read(asset).map_err(|error| error.to_string())?;
            if bytes.len() > 8 * 1024 * 1024 {
                return Err("UI background exceeds 8 MiB.".into());
            }
            let (width, height) = image::ImageReader::with_format(
                std::io::Cursor::new(&bytes),
                image::ImageFormat::Png,
            )
            .into_dimensions()
            .map_err(|error| error.to_string())?;
            if width > 4096 || height > 4096 {
                return Err("UI background must be at most 4096 × 4096.".into());
            }
            image::load_from_memory_with_format(&bytes, image::ImageFormat::Png)
                .map_err(|error| error.to_string())?;
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
    for (id, source) in [
        ("codex", include_str!("layouts/codex.json")),
        ("gemini", include_str!("layouts/gemini.json")),
    ] {
        packs.push(Pack {
            id: id.into(),
            config: serde_json::from_str(source).expect("Built-in UI config"),
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
    let mut config = read(&source)?;
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
        || ["codex", "gemini"].contains(&id.as_str())
    {
        id = format!("{slug}-{suffix}");
        suffix += 1;
    }
    let destination = directory.join(&id);
    std::fs::create_dir(&destination).map_err(|error| error.to_string())?;
    if let Some(asset) = &config.background_image {
        let asset = asset_path(&source, asset)?;
        std::fs::copy(asset, destination.join("background.png"))
            .map_err(|error| error.to_string())?;
        config.background_image = Some("background.png".into());
    }
    std::fs::write(
        destination.join("ui.json"),
        serde_json::to_vec_pretty(&config).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    Ok(id)
}

#[cfg(test)]
#[path = "../../tests/ui/packs.rs"]
mod tests;
