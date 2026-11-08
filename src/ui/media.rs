use std::ops::Range;
use std::path::{Path, PathBuf};

use eframe::egui;
use pulldown_cmark::{Event, Parser, Tag};

pub(super) struct Reference {
    pub range: Range<usize>,
    pub destination: String,
    pub video: bool,
}

pub(super) fn references(source: &str) -> Vec<Reference> {
    let mut references = Vec::new();
    let mut end = 0;
    for (event, range) in Parser::new(source).into_offset_iter() {
        if range.start < end {
            continue;
        }
        let (destination, image) = match event {
            Event::Start(Tag::Image { dest_url, .. }) => (dest_url, true),
            Event::Start(Tag::Link { dest_url, .. }) => (dest_url, false),
            _ => continue,
        };
        let destination = preserve_windows_path(&source[range.clone()], &destination);
        let extension = destination
            .split(['?', '#'])
            .next()
            .unwrap_or_default()
            .rsplit('.')
            .next()
            .unwrap_or_default()
            .to_ascii_lowercase();
        let video = matches!(
            extension.as_str(),
            "mp4" | "webm" | "mov" | "mkv" | "avi" | "m4v"
        );
        if image || video || matches!(extension.as_str(), "png" | "jpg" | "jpeg" | "gif" | "webp") {
            end = range.end;
            references.push(Reference {
                range,
                destination,
                video,
            });
        }
    }
    references
}

pub(super) fn preserve_windows_path(markdown: &str, parsed: &str) -> String {
    let raw = markdown
        .rsplit_once("](")
        .map(|(_, destination)| destination);
    let raw = raw.and_then(|destination| {
        if let Some(destination) = destination.strip_prefix('<') {
            destination
                .split_once('>')
                .map(|(destination, _)| destination)
        } else {
            destination
                .strip_suffix(')')
                .and_then(|destination| destination.split_whitespace().next())
        }
    });
    if let Some(raw) = raw
        && raw.trim_start_matches('/').as_bytes().get(1) == Some(&b':')
        && raw.contains('\\')
    {
        raw.replace('\\', "/")
    } else {
        parsed.to_owned()
    }
}

fn local_path(destination: &str, workspace: Option<&Path>) -> Result<PathBuf, String> {
    if destination.starts_with("file:") {
        return url::Url::parse(destination)
            .map_err(|error| error.to_string())?
            .to_file_path()
            .map_err(|_| "Invalid local media path.".into());
    }
    let destination = destination.strip_prefix("sandbox:").unwrap_or(destination);
    if destination.contains("://") {
        return Err("Unsupported media URL.".into());
    }
    let path = PathBuf::from(destination);
    Ok(if path.is_absolute() {
        path
    } else {
        workspace
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."))
            .join(path)
    })
}

fn locations(destination: &str, workspace: Option<&Path>) -> Result<(String, String), String> {
    if destination.starts_with("https://")
        || destination.starts_with("http://")
        || destination.starts_with("data:image/")
    {
        return Ok((destination.into(), destination.into()));
    }
    let path = local_path(destination, workspace)?;
    let path = if path.is_absolute() {
        path
    } else {
        std::env::current_dir()
            .map_err(|error| error.to_string())?
            .join(path)
    };
    let open_url = url::Url::from_file_path(&path)
        .map_err(|_| "Invalid media file path.")?
        .to_string();
    let native_path = path.to_string_lossy();
    let loader_path = if !cfg!(windows) || native_path.starts_with(r"\\") {
        native_path.into_owned()
    } else {
        native_path.replace('\\', "/")
    };
    Ok((format!("file://{loader_path}"), open_url))
}

pub(super) fn show(ui: &mut egui::Ui, destination: &str, video: bool, workspace: Option<&Path>) {
    let (uri, open_url) = match locations(destination, workspace) {
        Ok(locations) => locations,
        Err(error) => {
            ui.label(format!("Media unavailable: {error}"));
            return;
        }
    };
    if video {
        egui::Frame::group(ui.style()).show(ui, |ui| {
            open_button(ui, "▶ Play video", &open_url);
            ui.label("Opens in your default video player or browser.");
        });
    } else {
        egui_extras::install_image_loaders(ui.ctx());
        let image = egui::Image::new(uri.clone())
            .fit_to_original_size(1.0)
            .max_width(ui.available_width().min(720.0))
            .max_height(480.0);
        let loaded = image.load_for_size(ui.ctx(), ui.available_size());
        ui.add(image).on_hover_text(destination);
        if let Err(error) = loaded {
            ui.label(format!("Image unavailable: {error}"));
            if ui.small_button("Retry image").clicked() {
                ui.ctx().forget_image(&uri);
                ui.ctx().request_repaint();
            }
        }
        if !open_url.starts_with("data:") {
            open_button(ui, "Open image", &open_url);
        }
    }
    ui.add_space(8.0);
}

fn open_button(ui: &mut egui::Ui, label: &str, url: &str) {
    let id = ui.id().with(("media_open", url));
    if ui.button(label).clicked() {
        let error = redox::media::open_media(url).err();
        ui.data_mut(|data| data.insert_temp(id, error));
    }
    if let Some(Some(error)) = ui.data(|data| data.get_temp::<Option<String>>(id)) {
        ui.label(format!("Could not open media: {error}"));
    }
}

#[cfg(test)]
#[path = "../../tests/ui/media.rs"]
mod tests;
