use std::io::Read;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const MAX_ATTACHMENTS: usize = 8;
const MAX_TEXT_BYTES: u64 = 128 * 1024;
const MAX_IMAGE_BYTES: u64 = 20 * 1024 * 1024;

#[derive(Clone, Deserialize, Serialize)]
pub struct Attachment {
    pub id: Uuid,
    pub path: PathBuf,
    pub text: Option<String>,
}

impl Attachment {
    pub fn read(path: PathBuf) -> Result<Self, String> {
        let path = std::fs::canonicalize(&path)
            .map_err(|error| format!("Could not open {}: {error}", path.display()))?;
        let file = std::fs::File::open(&path)
            .map_err(|error| format!("Could not read {}: {error}", path.display()))?;
        let metadata = file.metadata().map_err(|error| error.to_string())?;
        if !metadata.is_file() {
            return Err("Select individual files, not directories.".into());
        }
        let extension = path
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();
        let image = matches!(extension.as_str(), "png" | "jpg" | "jpeg" | "webp" | "gif");
        let limit = if image {
            MAX_IMAGE_BYTES
        } else {
            MAX_TEXT_BYTES
        };
        if metadata.len() > limit {
            return Err(format!(
                "{} exceeds the {} KiB attachment limit.",
                path.display(),
                limit / 1024
            ));
        }
        let text = if image {
            None
        } else {
            let mut bytes = Vec::new();
            file.take(limit + 1)
                .read_to_end(&mut bytes)
                .map_err(|error| error.to_string())?;
            if bytes.len() as u64 > limit {
                return Err("The selected file grew beyond the attachment limit.".into());
            }
            let text = String::from_utf8(bytes)
                .map_err(|_| format!("{} is not UTF-8 text or a supported image. Attach a text/code file or PNG, JPEG, WebP, or GIF image.", path.display()))?;
            if text.contains('\0') {
                return Err(format!("{} appears to be a binary file.", path.display()));
            }
            Some(text.trim_start_matches('\u{feff}').to_owned())
        };
        Ok(Self {
            id: Uuid::new_v4(),
            path,
            text,
        })
    }

    pub fn name(&self) -> String {
        self.path
            .file_name()
            .unwrap_or(self.path.as_os_str())
            .to_string_lossy()
            .into_owned()
    }

    pub fn text_bytes(&self) -> usize {
        self.text.as_ref().map_or(0, String::len)
    }
}
