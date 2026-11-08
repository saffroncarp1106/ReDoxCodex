use eframe::egui::Color32;
use redox::desktop::Activity;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

pub(in crate::app) struct Style {
    pub colors: Vec<Color32>,
    pub names: Vec<String>,
    pub size: f32,
    pub show_names: bool,
    pub show_trails: bool,
}

pub(in crate::app) struct Overlay {
    activity: Activity,
    closed: Arc<AtomicBool>,
    worker: Option<std::thread::JoinHandle<()>>,
}

impl Overlay {
    pub fn start(activity: Activity, style: Style) -> Self {
        let closed = Arc::new(AtomicBool::new(false));
        let shutdown = closed.clone();
        let failure = activity.clone();
        let lifetime = activity.clone();
        let worker = std::thread::Builder::new()
            .name("desktop-feedback".into())
            .spawn(move || {
                #[cfg(windows)]
                if let Err(error) = windows::run(&activity, &style, &shutdown) {
                    eprintln!("Desktop feedback: {error}");
                    activity.stop();
                }
                #[cfg(not(windows))]
                let _ = (activity, style, shutdown);
            })
            .map_err(|error| {
                eprintln!("Desktop feedback worker: {error}");
                failure.stop();
            })
            .ok();
        Self {
            activity: lifetime,
            closed,
            worker,
        }
    }
}

impl Drop for Overlay {
    fn drop(&mut self) {
        self.activity.stop();
        self.closed.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

#[cfg(windows)]
mod windows {
    use super::*;
    use std::{
        collections::VecDeque,
        ptr::{null, null_mut},
        time::{Duration, Instant},
    };
    use windows_sys::Win32::{
        Foundation::{HWND, LPARAM, POINT, RECT},
        Graphics::Gdi::*,
        UI::WindowsAndMessaging::*,
    };

    fn error(operation: &str) -> String {
        format!("{operation}: {}", std::io::Error::last_os_error())
    }

    pub(super) struct Window {
        pub(super) handle: HWND,
        dc: HDC,
        buffer: HDC,
        bitmap: HBITMAP,
        previous: HGDIOBJ,
        rect: RECT,
        monitors: Vec<RECT>,
    }

    impl Drop for Window {
        fn drop(&mut self) {
            unsafe {
                if !self.buffer.is_null() {
                    SelectObject(self.buffer, self.previous);
                    DeleteDC(self.buffer);
                }
                if !self.bitmap.is_null() {
                    DeleteObject(self.bitmap);
                }
                if !self.dc.is_null() {
                    ReleaseDC(self.handle, self.dc);
                }
                if !self.handle.is_null() {
                    DestroyWindow(self.handle);
                }
            }
        }
    }

    unsafe extern "system" fn monitor(_: HMONITOR, _: HDC, rect: *mut RECT, data: LPARAM) -> i32 {
        unsafe {
            (&mut *(data as *mut Vec<RECT>)).push(*rect);
        }
        1
    }

    unsafe extern "system" fn window_proc(
        window: HWND,
        message: u32,
        wparam: usize,
        lparam: isize,
    ) -> isize {
        match message {
            WM_NCHITTEST => HTTRANSPARENT as isize,
            WM_MOUSEACTIVATE => MA_NOACTIVATE as isize,
            WM_ERASEBKGND => 1,
            WM_PAINT => {
                unsafe {
                    ValidateRect(window, null());
                }
                0
            }
            _ => unsafe { DefWindowProcW(window, message, wparam, lparam) },
        }
    }

    impl Window {
        pub(super) fn new() -> Result<Self, String> {
            let class: Vec<u16> = "STATIC\0".encode_utf16().collect();
            let title: Vec<u16> = "Codex desktop feedback\0".encode_utf16().collect();
            let rect = unsafe {
                RECT {
                    left: GetSystemMetrics(SM_XVIRTUALSCREEN),
                    top: GetSystemMetrics(SM_YVIRTUALSCREEN),
                    right: GetSystemMetrics(SM_XVIRTUALSCREEN)
                        + GetSystemMetrics(SM_CXVIRTUALSCREEN),
                    bottom: GetSystemMetrics(SM_YVIRTUALSCREEN)
                        + GetSystemMetrics(SM_CYVIRTUALSCREEN),
                }
            };
            let mut window = Self {
                handle: null_mut(),
                dc: null_mut(),
                buffer: null_mut(),
                bitmap: null_mut(),
                previous: null_mut(),
                rect,
                monitors: Vec::new(),
            };
            unsafe {
                window.handle = CreateWindowExW(
                    WS_EX_TOPMOST
                        | WS_EX_LAYERED
                        | WS_EX_TRANSPARENT
                        | WS_EX_NOACTIVATE
                        | WS_EX_TOOLWINDOW,
                    class.as_ptr(),
                    title.as_ptr(),
                    WS_POPUP,
                    rect.left,
                    rect.top,
                    rect.right - rect.left,
                    rect.bottom - rect.top,
                    null_mut(),
                    null_mut(),
                    null_mut(),
                    null(),
                );
                if window.handle.is_null() {
                    return Err(error("Create desktop overlay"));
                }
                SetWindowLongPtrW(
                    window.handle,
                    GWLP_WNDPROC,
                    window_proc as *const () as isize,
                );
                if SetLayeredWindowAttributes(window.handle, 0, 255, LWA_COLORKEY) == 0 {
                    return Err(error("Make desktop overlay transparent"));
                }
                if SetWindowDisplayAffinity(window.handle, WDA_EXCLUDEFROMCAPTURE) == 0 {
                    return Err(error("Exclude desktop overlay from screenshots"));
                }
                EnumDisplayMonitors(
                    null_mut(),
                    null(),
                    Some(monitor),
                    &mut window.monitors as *mut _ as LPARAM,
                );
                if window.monitors.is_empty() {
                    window.monitors.push(rect);
                }
                window.dc = GetDC(window.handle);
                window.buffer = CreateCompatibleDC(window.dc);
                window.bitmap = CreateCompatibleBitmap(
                    window.dc,
                    rect.right - rect.left,
                    rect.bottom - rect.top,
                );
                if window.dc.is_null() || window.buffer.is_null() || window.bitmap.is_null() {
                    return Err(error("Create desktop overlay buffer"));
                }
                window.previous = SelectObject(window.buffer, window.bitmap);
            }
            Ok(window)
        }

        pub(super) fn paint(
            &self,
            style: &Style,
            trail: &VecDeque<POINT>,
            elapsed: f32,
            click: Option<(POINT, Instant)>,
        ) {
            let dc = self.buffer;
            let color = style
                .colors
                .first()
                .copied()
                .unwrap_or(Color32::from_rgb(140, 105, 184));
            let width = self.rect.right - self.rect.left;
            let height = self.rect.bottom - self.rect.top;
            unsafe {
                PatBlt(dc, 0, 0, width, height, BLACKNESS);
                SetBkMode(dc, TRANSPARENT as i32);
            }
            for monitor in &self.monitors {
                let rect = RECT {
                    left: monitor.left - self.rect.left,
                    top: monitor.top - self.rect.top,
                    right: monitor.right - self.rect.left,
                    bottom: monitor.bottom - self.rect.top,
                };
                let pulse = 0.7 + 0.3 * (elapsed * 2.4).sin().abs();
                for inset in (1..=10).rev() {
                    stroke(
                        dc,
                        2,
                        color.gamma_multiply(pulse * (11 - inset) as f32 / 10.0),
                        || unsafe {
                            Rectangle(
                                dc,
                                rect.left + inset,
                                rect.top + inset,
                                rect.right - inset,
                                rect.bottom - inset,
                            );
                        },
                    );
                }
                label(
                    dc,
                    (rect.left + rect.right) / 2 - 140,
                    rect.top + 18,
                    "ReDox remote control  |  Esc to stop",
                    Color32::WHITE,
                );
            }
            if let Some((point, started)) = click {
                let age = started.elapsed().as_secs_f32();
                if age < 0.65 {
                    let radius = (12.0 + age * 50.0) as i32;
                    stroke(dc, 3, color, || unsafe {
                        Ellipse(
                            dc,
                            point.x - radius,
                            point.y - radius,
                            point.x + radius,
                            point.y + radius,
                        );
                    });
                }
            }
            if style.show_trails {
                for (index, (from, to)) in trail.iter().zip(trail.iter().skip(1)).enumerate() {
                    let fade = (index + 1) as f32 / trail.len().max(1) as f32;
                    stroke(dc, 2, color.gamma_multiply(fade * 0.7), || unsafe {
                        MoveToEx(dc, from.x, from.y, null_mut());
                        LineTo(dc, to.x, to.y);
                    });
                }
            }
            if let Some(point) = trail.back() {
                let points = super::super::startup::cursor_points(
                    eframe::egui::pos2(point.x as f32, point.y as f32),
                    style.size,
                );
                let points = points.map(|point| POINT {
                    x: point.x.round() as i32,
                    y: point.y.round() as i32,
                });
                unsafe {
                    let brush = CreateSolidBrush(rgb(color));
                    let previous = SelectObject(dc, brush);
                    let pen = CreatePen(PS_SOLID, 2, rgb(Color32::WHITE));
                    let old_pen = SelectObject(dc, pen);
                    Polygon(dc, points.as_ptr(), points.len() as i32);
                    SelectObject(dc, previous);
                    SelectObject(dc, old_pen);
                    DeleteObject(brush);
                    DeleteObject(pen);
                }
                if style.show_names {
                    let name = style.names.first().map(String::as_str).unwrap_or("Codex");
                    label(dc, point.x + 22, point.y + 22, name, color);
                }
            }
            unsafe {
                BitBlt(self.dc, 0, 0, width, height, dc, 0, 0, SRCCOPY);
            }
        }

        #[cfg(test)]
        pub(super) fn preview(&self) -> image::RgbaImage {
            let width = (self.rect.right - self.rect.left) as u32;
            let height = (self.rect.bottom - self.rect.top) as u32;
            let mut info = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: width as i32,
                    biHeight: -(height as i32),
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB,
                    ..Default::default()
                },
                ..Default::default()
            };
            let mut pixels = vec![0u8; width as usize * height as usize * 4];
            unsafe {
                SelectObject(self.buffer, self.previous);
                let rows = GetDIBits(
                    self.dc,
                    self.bitmap,
                    0,
                    height,
                    pixels.as_mut_ptr().cast(),
                    &mut info,
                    DIB_RGB_COLORS,
                );
                SelectObject(self.buffer, self.bitmap);
                assert_eq!(rows, height as i32);
            }
            for pixel in pixels.chunks_exact_mut(4) {
                pixel.swap(0, 2);
                pixel[3] = 255;
            }
            image::RgbaImage::from_raw(width, height, pixels).unwrap()
        }
    }

    fn rgb(color: Color32) -> u32 {
        color.r() as u32 | ((color.g() as u32) << 8) | ((color.b() as u32) << 16)
    }

    fn stroke(dc: HDC, width: i32, color: Color32, paint: impl FnOnce()) {
        unsafe {
            let pen = CreatePen(PS_SOLID, width, rgb(color));
            let previous = SelectObject(dc, pen);
            let brush = SelectObject(dc, GetStockObject(NULL_BRUSH));
            paint();
            SelectObject(dc, previous);
            SelectObject(dc, brush);
            DeleteObject(pen);
        }
    }

    fn label(dc: HDC, x: i32, y: i32, text: &str, color: Color32) {
        let text: Vec<u16> = text.encode_utf16().collect();
        unsafe {
            SetTextColor(dc, rgb(Color32::from_rgb(20, 20, 24)));
            TextOutW(dc, x + 1, y + 1, text.as_ptr(), text.len() as i32);
            SetTextColor(dc, rgb(color));
            TextOutW(dc, x, y, text.as_ptr(), text.len() as i32);
        }
    }

    pub(super) fn run(
        activity: &Activity,
        style: &Style,
        closed: &AtomicBool,
    ) -> Result<(), String> {
        let mut window = None;
        let started = Instant::now();
        let mut trail = VecDeque::new();
        let mut clicks = 0;
        let mut click = None;
        while !closed.load(Ordering::Acquire) {
            let feedback = activity.snapshot();
            if feedback.stopped {
                break;
            }
            if feedback.active {
                if window.is_none() {
                    window = Some(Window::new()?);
                }
                let window = window.as_ref().unwrap();
                let mut point = POINT::default();
                if unsafe { GetCursorPos(&mut point) } != 0 {
                    point.x -= window.rect.left;
                    point.y -= window.rect.top;
                    trail.push_back(point);
                    if trail.len() > 18 {
                        trail.pop_front();
                    }
                    if clicks != feedback.clicks {
                        clicks = feedback.clicks;
                        click = Some((point, Instant::now()));
                    }
                }
                window.paint(style, &trail, started.elapsed().as_secs_f32(), click);
                unsafe {
                    SetWindowPos(
                        window.handle,
                        HWND_TOPMOST,
                        0,
                        0,
                        0,
                        0,
                        SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW,
                    );
                }
            } else if let Some(window) = window.as_ref() {
                unsafe {
                    ShowWindow(window.handle, SW_HIDE);
                }
            }
            let mut message = MSG::default();
            while unsafe { PeekMessageW(&mut message, null_mut(), 0, 0, PM_REMOVE) } != 0 {
                unsafe {
                    TranslateMessage(&message);
                    DispatchMessageW(&message);
                }
            }
            std::thread::sleep(Duration::from_millis(16));
        }
        Ok(())
    }
}

#[cfg(all(test, windows))]
#[path = "../../tests/ui/desktop_overlay.rs"]
mod tests;
