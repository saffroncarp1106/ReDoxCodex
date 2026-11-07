use super::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

#[test]
fn overlay_is_click_through_excluded_from_capture_and_does_not_take_focus() {
    let foreground = unsafe { GetForegroundWindow() };
    let window = windows::Window::new().unwrap();
    let styles = unsafe { GetWindowLongW(window.handle, GWL_EXSTYLE) } as u32;
    for required in [
        WS_EX_LAYERED,
        WS_EX_TRANSPARENT,
        WS_EX_NOACTIVATE,
        WS_EX_TOOLWINDOW,
    ] {
        assert_eq!(styles & required, required);
    }
    let mut affinity = 0;
    assert_ne!(
        unsafe { GetWindowDisplayAffinity(window.handle, &mut affinity) },
        0
    );
    assert_eq!(affinity, WDA_EXCLUDEFROMCAPTURE);
    assert_eq!(unsafe { GetForegroundWindow() }, foreground);
    assert_eq!(
        unsafe { SendMessageW(window.handle, WM_NCHITTEST, 0, 0) },
        HTTRANSPARENT as isize
    );
}

#[test]
fn native_feedback_paints_the_swarm_cursor_and_leaves_the_desktop_transparent() {
    use windows_sys::Win32::Foundation::POINT;
    let window = windows::Window::new().unwrap();
    let style = Style {
        colors: vec![Color32::from_rgb(140, 105, 184)],
        names: vec!["Codex".into()],
        size: 24.0,
        show_names: true,
        show_trails: true,
    };
    let trail = (0..18)
        .map(|index| POINT {
            x: 250 + index * 6,
            y: 200 + index * 3,
        })
        .collect();
    window.paint(&style, &trail, 0.0, None);
    let image = window.preview();
    assert_eq!(image.get_pixel(100, 100).0, [0, 0, 0, 255]);
    assert_ne!(image.get_pixel(356, 257).0, [0, 0, 0, 255]);
    if let Some(path) = std::env::var_os("REDOX_OVERLAY_PREVIEW") {
        image.save(path).unwrap();
    }
}
