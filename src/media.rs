use std::hash::{Hash, Hasher};
use std::path::PathBuf;

use base64::Engine;
use serde_json::Value;

pub fn open_media(destination: &str) -> Result<(), String> {
    let url = url::Url::parse(destination).map_err(|error| error.to_string())?;
    match url.scheme() {
        "file" => {
            let path = url.to_file_path().map_err(|_| "Invalid media path.")?;
            open::that_detached(path).map_err(|error| error.to_string())
        }
        "http" | "https" => open::that_detached(destination).map_err(|error| error.to_string()),
        _ => Err("Unsupported media URL.".into()),
    }
}

pub fn image_markdown(source: &str) -> String {
    let source = if source.as_bytes().get(1) == Some(&b':') {
        source.replace('\\', "/")
    } else {
        source.to_owned()
    };
    format!(
        "\n\n![Image](<{}>)\n\n",
        source.replace('>', "%3E").replace(['\r', '\n'], "")
    )
}

pub fn generated_image(item: &Value) -> Option<String> {
    if item["type"] != "imageGeneration" {
        return None;
    }
    if let Some(path) = item["savedPath"].as_str().filter(|path| !path.is_empty()) {
        return Some(image_markdown(path));
    }
    if let Some(result) = item["result"].as_str().filter(|value| !value.is_empty()) {
        if result.starts_with("https://")
            || result.starts_with("http://")
            || result.starts_with("file://")
        {
            return Some(image_markdown(result));
        }
        return Some(match save_inline_image(result) {
            Ok(path) => image_markdown(&path.to_string_lossy()),
            Err(error) => format!("\n\nGenerated image could not be loaded: {error}\n\n"),
        });
    }
    if item["failure"].is_object() || item["status"] == "failed" {
        return Some("\n\nImage generation failed; no image was returned.\n\n".into());
    }
    Some("\n\nGenerated image has no local file or image data available from Codex.\n\n".into())
}

fn decode_inline_image(result: &str) -> Result<Vec<u8>, String> {
    let encoded = if result.starts_with("data:") {
        let (header, data) = result.split_once(',').ok_or("Invalid image data URL.")?;
        if !header.starts_with("data:image/") || !header.ends_with(";base64") {
            return Err("Unsupported image data URL.".into());
        }
        data
    } else {
        result
    };
    if encoded.len() > 28 * 1024 * 1024 {
        return Err("Image exceeds the 20 MiB storage limit.".into());
    }
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .map_err(|error| error.to_string())?;
    if bytes.len() > 20 * 1024 * 1024 {
        return Err("Image exceeds the 20 MiB storage limit.".into());
    }
    Ok(bytes)
}

fn save_inline_image(result: &str) -> Result<PathBuf, String> {
    let bytes = decode_inline_image(result)?;
    let format = image::guess_format(&bytes).map_err(|error| error.to_string())?;
    let extension = match format {
        image::ImageFormat::Png => "png",
        image::ImageFormat::Jpeg => "jpg",
        image::ImageFormat::Gif => "gif",
        image::ImageFormat::WebP => "webp",
        _ => return Err("Unsupported generated-image format.".into()),
    };
    let root = dirs::data_local_dir()
        .ok_or("Local application data folder is unavailable.")?
        .join("ReDox/media");
    std::fs::create_dir_all(&root).map_err(|error| error.to_string())?;
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    bytes.hash(&mut hash);
    let path = root.join(format!("{:016x}.{extension}", hash.finish()));
    if !path.is_file() {
        std::fs::write(&path, bytes).map_err(|error| error.to_string())?;
    }
    Ok(path)
}

#[cfg(test)]
#[path = "../tests/transport/media.rs"]
mod tests;
