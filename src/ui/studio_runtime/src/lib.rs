#[cfg(test)]
extern crate self as redox_studio;

use serde::{Deserialize, Serialize};
use std::{fs::File, io::Read, path::Path};

pub mod actions;
pub mod assets;
pub mod document;
pub mod editor;
pub mod originals;
pub mod presentation;
pub mod scenes;
pub mod simulation;

pub const MAX_BYTES: usize = 8192;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Skin {
    pub version: u8,
    pub name: String,
    pub author: String,
    pub license: License,
    pub palette: Palette,
    pub layout: Layout,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum License {
    #[serde(rename = "CC0-1.0")]
    Cc0,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Palette {
    pub background: [u8; 3],
    pub surface: [u8; 3],
    pub text: [u8; 3],
    pub accent: [u8; 3],
    pub accent_text: [u8; 3],
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Layout {
    pub sidebar_width: u16,
    pub spacing: u8,
    pub text_size: u8,
    pub rounding: u8,
    pub bevel: bool,
    pub sidebar_right: bool,
}

impl Skin {
    pub fn modern() -> Self {
        Self {
            version: 1,
            name: "Quiet light".into(),
            author: "Community".into(),
            license: License::Cc0,
            palette: Palette {
                background: [239, 241, 245],
                surface: [255, 255, 255],
                text: [32, 37, 49],
                accent: [62, 69, 185],
                accent_text: [255, 255, 255],
            },
            layout: Layout {
                sidebar_width: 180,
                spacing: 16,
                text_size: 15,
                rounding: 10,
                bevel: false,
                sidebar_right: false,
            },
        }
    }
    pub fn classic() -> Self {
        Self {
            name: "Classic 98".into(),
            palette: Palette {
                background: [0, 128, 128],
                surface: [192, 192, 192],
                text: [0, 0, 0],
                accent: [0, 0, 128],
                accent_text: [255, 255, 255],
            },
            layout: Layout {
                rounding: 0,
                bevel: true,
                spacing: 12,
                ..Self::modern().layout
            },
            ..Self::modern()
        }
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.version != 1 {
            return Err("Unsupported skin version".into());
        }
        for (label, value) in [("Name", &self.name), ("Author", &self.author)] {
            if value.trim().is_empty()
                || value.len() > 48
                || !value
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || " -_".contains(c))
            {
                return Err(format!(
                    "{label}: use 1–48 letters, numbers, spaces, hyphens or underscores"
                ));
            }
        }
        let l = &self.layout;
        if !(120..=260).contains(&l.sidebar_width)
            || !(8..=32).contains(&l.spacing)
            || !(12..=24).contains(&l.text_size)
            || l.rounding > 24
        {
            return Err("Layout value outside supported range".into());
        }
        if contrast(self.palette.text, self.palette.surface) < 4.5
            || contrast(self.palette.accent_text, self.palette.accent) < 4.5
        {
            return Err("Text contrast must be at least 4.5:1".into());
        }
        Ok(())
    }
    pub fn parse(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() > MAX_BYTES {
            return Err("Skin exceeds 8 KiB".into());
        }
        let skin: Self = serde_json::from_slice(bytes).map_err(|e| format!("Invalid skin: {e}"))?;
        skin.validate()?;
        Ok(skin)
    }
    pub fn load(path: &Path) -> Result<Self, String> {
        if path.extension().and_then(|s| s.to_str()) != Some("redoxskin") {
            return Err("Choose a .redoxskin file".into());
        }
        let mut bytes = Vec::new();
        File::open(path)
            .map_err(|e| e.to_string())?
            .take((MAX_BYTES + 1) as u64)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        Self::parse(&bytes)
    }
    pub fn encode(&self) -> Result<Vec<u8>, String> {
        self.validate()?;
        serde_json::to_vec_pretty(self).map_err(|e| e.to_string())
    }
}

pub fn contrast(a: [u8; 3], b: [u8; 3]) -> f32 {
    fn luminance(rgb: [u8; 3]) -> f32 {
        let c = rgb.map(|v| {
            let v = v as f32 / 255.;
            if v <= 0.04045 {
                v / 12.92
            } else {
                ((v + 0.055) / 1.055).powf(2.4)
            }
        });
        c[0] * 0.2126 + c[1] * 0.7152 + c[2] * 0.0722
    }
    let (a, b) = (luminance(a), luminance(b));
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}

#[cfg(test)]
#[path = "../tests/ui_scripts.rs"]
mod ui_scripts;

#[cfg(test)]
#[path = "../tests/playground.rs"]
mod playground_tests;

#[cfg(test)]
#[path = "../tests/source_pixels.rs"]
mod source_pixel_tests;
