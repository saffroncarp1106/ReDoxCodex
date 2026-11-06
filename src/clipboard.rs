use std::path::PathBuf;

pub enum Payload {
    Files(Vec<PathBuf>),
    Image(arboard::ImageData<'static>),
}

pub fn capture() -> Result<Option<Payload>, String> {
    let mut clipboard = arboard::Clipboard::new().map_err(|error| error.to_string())?;
    if let Ok(paths) = clipboard.get().file_list()
        && !paths.is_empty()
    {
        return Ok(Some(Payload::Files(paths)));
    }
    match clipboard.get_image() {
        Ok(image) => Ok(Some(Payload::Image(image))),
        Err(arboard::Error::ContentNotAvailable) => Ok(None),
        Err(error) => Err(error.to_string()),
    }
}

pub fn attachment_paths(payload: Payload) -> Result<Vec<PathBuf>, String> {
    match payload {
        Payload::Files(paths) => Ok(paths),
        Payload::Image(image) => {
            let pixels = image
                .width
                .checked_mul(image.height)
                .ok_or("Clipboard image dimensions overflow.")?;
            if pixels == 0 || pixels > 16 * 1024 * 1024 {
                return Err("Clipboard image exceeds the 16-megapixel limit.".into());
            }
            let width = u32::try_from(image.width).map_err(|_| "Image width is too large.")?;
            let height = u32::try_from(image.height).map_err(|_| "Image height is too large.")?;
            let image = image::RgbaImage::from_raw(width, height, image.bytes.into_owned())
                .ok_or("Invalid clipboard image pixels.")?;
            let root = dirs::data_local_dir()
                .ok_or("Application data folder unavailable.")?
                .join("ForgeDesk/pasted-images");
            std::fs::create_dir_all(&root).map_err(|error| error.to_string())?;
            let path = root.join(format!("clipboard-{}.png", uuid::Uuid::new_v4()));
            image.save(&path).map_err(|error| error.to_string())?;
            Ok(vec![path])
        }
    }
}

#[cfg(test)]
#[path = "../tests/transport/clipboard.rs"]
mod tests;
