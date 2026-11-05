use super::*;
use serde_json::json;

#[test]
fn macos_and_linux_image_paths_are_preserved() {
    for path in [
        "/Users/name/.codex/generated_images/dog.png",
        "/home/name/.codex/generated_images/dog.png",
    ] {
        let item = json!({"type":"imageGeneration","status":"completed","savedPath":path});
        assert!(
            generated_image(&item)
                .unwrap()
                .contains(&format!("<{path}>"))
        );
    }
}

#[test]
fn generated_windows_image_paths_are_markdown_safe() {
    let item = json!({"type":"imageGeneration","status":"completed","savedPath":r"C:\Users\someone\.codex\generated_images\dog.png"});
    assert!(
        generated_image(&item)
            .unwrap()
            .contains("<C:/Users/someone/.codex/generated_images/dog.png>")
    );
}

#[test]
fn generated_images_keep_the_saved_path_in_live_and_history_content() {
    let item = json!({"type":"imageGeneration","id":"image-1","status":"completed","savedPath":"C:/Project files/dog.png","result":"ignored"});
    let content = generated_image(&item).unwrap();
    assert!(content.contains("![Image](<C:/Project files/dog.png>)"));
    let page = json!({"data":[{"status":"completed","items":[item]}]});
    let messages = crate::history::parse_turns(&page, "test").unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].content, content);
}

#[test]
fn inline_images_accept_base64_and_data_urls_and_reject_invalid_input() {
    let bytes = b"fixture bytes";
    let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
    assert_eq!(decode_inline_image(&encoded).unwrap(), bytes);
    assert_eq!(
        decode_inline_image(&format!("data:image/png;base64,{encoded}")).unwrap(),
        bytes
    );
    assert!(decode_inline_image("invalid!!!").is_err());
    assert!(decode_inline_image("data:text/html;base64,aGVsbG8=").is_err());
}

#[test]
fn unavailable_images_are_reported_instead_of_silently_discarded() {
    assert!(generated_image(&json!({"type":"agentMessage"})).is_none());
    assert!(
        generated_image(&json!({"type":"imageGeneration","status":"failed"}))
            .unwrap()
            .contains("failed")
    );
    assert!(
        generated_image(&json!({"type":"imageGeneration","status":"completed","fileId":"file-1"}))
            .unwrap()
            .contains("no local file")
    );
}
