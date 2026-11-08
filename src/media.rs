use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use base64::Engine;
use serde_json::Value;

pub fn open_media(destination: &str) -> Result<(), String> {
    let url = url::Url::parse(destination).map_err(|error| error.to_string())?;
    match url.scheme() {
        "file" => open_file_link(destination, None),
        "http" | "https" => open::that_detached(destination).map_err(|error| error.to_string()),
        _ => Err("Unsupported media URL.".into()),
    }
}

pub fn is_file_link(destination: &str) -> bool {
    if destination.is_empty() || destination.starts_with('#') {
        return false;
    }
    let path = strip_line_numbers(destination.split('#').next().unwrap_or(destination));
    if is_windows_drive(path) {
        return true;
    }
    match url::Url::parse(path) {
        Ok(url) => url.scheme() == "file",
        Err(url::ParseError::RelativeUrlWithoutBase) => true,
        Err(_) => false,
    }
}

pub fn open_file_link(destination: &str, workspace: Option<&Path>) -> Result<(), String> {
    let path = resolve_file_link(destination, workspace)?;
    open::that_detached(&path).map_err(|error| format!("{}: {error}", path.display()))
}

fn resolve_file_link(destination: &str, workspace: Option<&Path>) -> Result<PathBuf, String> {
    if !is_file_link(destination) {
        return Err("Unsupported file link.".into());
    }
    let without_fragment = destination
        .split_once('#')
        .map_or(destination, |(path, _)| path);
    let without_position = strip_line_numbers(without_fragment);
    let mut last_error = String::new();
    for target in std::iter::once(destination)
        .chain((without_position != destination).then_some(without_position))
    {
        let decoded = percent_encoding::percent_decode_str(target).decode_utf8_lossy();
        let encoded_path = !target
            .get(..5)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("file:"))
            && decoded != target;
        for candidate in std::iter::once(target).chain(encoded_path.then_some(decoded.as_ref())) {
            let path = file_link_path(candidate, workspace)?;
            match std::fs::metadata(&path) {
                Ok(_) => return Ok(path),
                Err(error) => last_error = format!("{}: {error}", path.display()),
            }
        }
    }
    Err(last_error)
}

fn file_link_path(target: &str, workspace: Option<&Path>) -> Result<PathBuf, String> {
    if target
        .get(..5)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("file:"))
    {
        return url::Url::parse(target)
            .ok()
            .and_then(|url| url.to_file_path().ok())
            .ok_or_else(|| "Invalid file URL.".into());
    }
    let path = target
        .strip_prefix('/')
        .filter(|path| cfg!(windows) && is_windows_drive(path))
        .unwrap_or(target);
    let path = Path::new(path);
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        Ok(workspace
            .ok_or("This relative link needs a project folder.")?
            .join(path))
    }
}

fn is_windows_drive(path: &str) -> bool {
    matches!(path.as_bytes(), [letter, b':', b'/' | b'\\', ..] if letter.is_ascii_alphabetic())
}

fn strip_line_numbers(mut path: &str) -> &str {
    for _ in 0..2 {
        if let Some((name, suffix)) = path.rsplit_once(':')
            && !suffix.is_empty()
            && suffix.bytes().all(|byte| byte.is_ascii_digit())
        {
            path = name;
        } else {
            break;
        }
    }
    path
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
