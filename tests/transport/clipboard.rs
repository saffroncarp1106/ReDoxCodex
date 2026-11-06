use super::*;

#[test]
fn pasted_file_lists_use_the_existing_attachment_snapshot_reader() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("temp/verification/clipboard")
        .join(uuid::Uuid::new_v4().to_string());
    std::fs::create_dir_all(&root).unwrap();
    let path = root.join("pasted file.txt");
    std::fs::write(&path, "CLIPBOARD_FILE_SNAPSHOT").unwrap();
    let paths = attachment_paths(Payload::Files(vec![path.clone()])).unwrap();
    let attachment = crate::attachments::Attachment::read(paths[0].clone()).unwrap();
    std::fs::write(path, "changed later").unwrap();
    assert_eq!(attachment.text.as_deref(), Some("CLIPBOARD_FILE_SNAPSHOT"));
}

#[test]
fn clipboard_images_are_saved_as_attachable_pngs_and_invalid_pixels_are_rejected() {
    let image = arboard::ImageData {
        width: 2,
        height: 2,
        bytes: vec![255; 16].into(),
    };
    let paths = attachment_paths(Payload::Image(image)).unwrap();
    let attachment = crate::attachments::Attachment::read(paths[0].clone()).unwrap();
    assert!(attachment.text.is_none());
    assert_eq!(image::open(&attachment.path).unwrap().width(), 2);
    assert!(
        attachment_paths(Payload::Image(arboard::ImageData {
            width: 2,
            height: 2,
            bytes: vec![0; 3].into()
        }))
        .is_err()
    );
    assert!(
        attachment_paths(Payload::Image(arboard::ImageData {
            width: usize::MAX,
            height: 2,
            bytes: Vec::new().into()
        }))
        .is_err()
    );
}
