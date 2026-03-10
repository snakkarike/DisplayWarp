use eframe::egui;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;

use parking_lot::Mutex;
use tray_icon::{Icon, TrayIcon, TrayIconBuilder, TrayIconEvent};

use crate::models::SavedData;

fn tray_icon() -> Icon {
    let (rgba, w, h) =
        crate::svg_render::png_to_rgba(include_bytes!("../assets/DisplayWarpIcon64.png"));
    Icon::from_rgba(rgba, w, h).expect("failed to create tray icon")
}

fn is_dark_mode() -> bool {
    use windows::Win32::System::Registry::{
        HKEY_CURRENT_USER, KEY_READ, RegCloseKey, RegOpenKeyExW, RegQueryValueExW,
    };
    use windows::core::PCWSTR;
    unsafe {
        let subkey: Vec<u16> =
            "Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize\0"
                .encode_utf16()
                .collect();
        let value_name: Vec<u16> = "AppsUseLightTheme\0".encode_utf16().collect();
        let mut hkey = windows::Win32::System::Registry::HKEY::default();
        if RegOpenKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR(subkey.as_ptr()),
            Some(0),
            KEY_READ,
            &mut hkey,
        )
        .is_err()
        {
            return false;
        }
        let mut data: u32 = 0;
        let mut data_size: u32 = std::mem::size_of::<u32>() as u32;
        let ok = RegQueryValueExW(
            hkey,
            PCWSTR(value_name.as_ptr()),
            None,
            None,
            Some(&mut data as *mut u32 as *mut u8),
            Some(&mut data_size),
        )
        .is_ok();
        let _ = RegCloseKey(hkey);
        if ok { data == 0 } else { false }
    }
}

mod uxtheme {
    use windows::Win32::Foundation::HMODULE;
    use windows::core::BOOL;

    type FnAllowDarkModeForWindow =
        unsafe extern "system" fn(windows::Win32::Foundation::HWND, BOOL) -> BOOL;
    type FnFlushMenuThemes = unsafe extern "system" fn();
    type FnSetPreferredAppMode = unsafe extern "system" fn(i32) -> i32;

    fn load_uxtheme() -> HMODULE {
        use windows::core::PCSTR;
        unsafe {
            windows::Win32::System::LibraryLoader::LoadLibraryA(PCSTR(b"uxtheme.dll\0".as_ptr()))
                .unwrap_or_default()
        }
    }

    pub fn allow_dark_mode_for_window(hwnd: windows::Win32::Foundation::HWND, allow: bool) {
        unsafe {
            let h = load_uxtheme();
            if h.is_invalid() {
                return;
            }
            let f: Option<FnAllowDarkModeForWindow> =
                std::mem::transmute(windows::Win32::System::LibraryLoader::GetProcAddress(
                    h,
                    windows::core::PCSTR(133usize as *const u8),
                ));
            if let Some(f) = f {
                let _ = f(hwnd, BOOL(allow as i32));
            }
        }
    }

    pub fn set_preferred_app_mode(mode: i32) {
        unsafe {
            let h = load_uxtheme();
            if h.is_invalid() {
                return;
            }
            let f: Option<FnSetPreferredAppMode> =
                std::mem::transmute(windows::Win32::System::LibraryLoader::GetProcAddress(
                    h,
                    windows::core::PCSTR(135usize as *const u8),
                ));
            if let Some(f) = f {
                f(mode);
            }
        }
    }

    pub fn flush_menu_themes() {
        unsafe {
            let h = load_uxtheme();
            if h.is_invalid() {
                return;
            }
            let f: Option<FnFlushMenuThemes> =
                std::mem::transmute(windows::Win32::System::LibraryLoader::GetProcAddress(
                    h,
                    windows::core::PCSTR(136usize as *const u8),
                ));
            if let Some(f) = f {
                f();
            }
        }
    }
}

fn apply_window_dark_mode(dark: bool) {
    unsafe {
        use windows::Win32::Graphics::Dwm::{DWMWA_USE_IMMERSIVE_DARK_MODE, DwmSetWindowAttribute};
        use windows::Win32::UI::WindowsAndMessaging::FindWindowW;
        use windows::core::w;
        if let Ok(hwnd) = FindWindowW(None, w!("DisplayWarp")) {
            if !hwnd.0.is_null() {
                uxtheme::allow_dark_mode_for_window(hwnd, dark);
                let value: u32 = dark as u32;
                let _ = DwmSetWindowAttribute(
                    hwnd,
                    DWMWA_USE_IMMERSIVE_DARK_MODE,
                    &value as *const u32 as *const _,
                    std::mem::size_of::<u32>() as u32,
                );
            }
        }
    }
}

fn cursor_pos() -> (i32, i32) {
    unsafe {
        use windows::Win32::Foundation::POINT;
        use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;
        let mut pt = POINT { x: 0, y: 0 };
        let _ = GetCursorPos(&mut pt);
        (pt.x, pt.y)
    }
}

fn dpi_scale_at(x: i32, y: i32) -> f32 {
    unsafe {
        use windows::Win32::Foundation::POINT;
        use windows::Win32::Graphics::Gdi::{MONITOR_DEFAULTTONEAREST, MonitorFromPoint};
        use windows::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};
        let pt = POINT { x, y };
        let monitor = MonitorFromPoint(pt, MONITOR_DEFAULTTONEAREST);
        let mut dpi_x: u32 = 96;
        let mut dpi_y: u32 = 96;
        let _ = GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y);
        dpi_x as f32 / 96.0
    }
}

pub struct PopupRequest {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub dark: bool,
}

pub struct TrayItems {
    _tray: TrayIcon,
    pub popup_rx: mpsc::Receiver<PopupRequest>,
}

impl TrayItems {
    pub fn refresh_menu(
        &self,
        _profiles: &[crate::models::AppProfile],
        _layouts: &[crate::models::SavedDisplayLayout],
    ) {
        let dark = is_dark_mode();
        if dark {
            uxtheme::set_preferred_app_mode(2);
        } else {
            uxtheme::set_preferred_app_mode(1);
        }
        uxtheme::flush_menu_themes();
        apply_window_dark_mode(dark);
    }
}

pub fn create_tray(
    watcher_running: Arc<AtomicBool>,
    data: Arc<Mutex<SavedData>>,
    status_message: Arc<parking_lot::Mutex<String>>,
    status_log: Arc<parking_lot::Mutex<Vec<String>>>,
    ctx: egui::Context, // <-- takes egui context to wake up the main loop
) -> TrayItems {
    let dark = is_dark_mode();
    if dark {
        uxtheme::set_preferred_app_mode(2);
    } else {
        uxtheme::set_preferred_app_mode(1);
    }
    uxtheme::flush_menu_themes();

    let icon = tray_icon();
    let tray = TrayIconBuilder::new()
        .with_tooltip("DisplayWarp")
        .with_icon(icon)
        .build()
        .expect("failed to build tray icon");

    let (popup_tx, popup_rx) = mpsc::channel::<PopupRequest>();

    std::thread::spawn(move || {
        let tray_receiver = TrayIconEvent::receiver();
        loop {
            if let Ok(event) = tray_receiver.recv() {
                let is_click = matches!(
                    event,
                    tray_icon::TrayIconEvent::Click {
                        button: tray_icon::MouseButton::Left,
                        button_state: tray_icon::MouseButtonState::Up,
                        ..
                    } | tray_icon::TrayIconEvent::Click {
                        button: tray_icon::MouseButton::Right,
                        button_state: tray_icon::MouseButtonState::Up,
                        ..
                    }
                );
                if !is_click {
                    continue;
                }

                let (cx, cy) = cursor_pos();
                let scale = dpi_scale_at(cx, cy);
                let dark = is_dark_mode();

                let popup_w = 230.0f32;
                let popup_h = {
                    let d = data.lock();
                    let items = d.profiles.len().max(1) + d.display_profiles.len().max(1);
                    (items as f32 * 28.0) + (2.0 * 24.0) + (3.0 * 9.0) + 28.0 + 28.0 + 12.0
                };

                let lx = cx as f32 / scale;
                let ly = cy as f32 / scale;

                let screen_w = unsafe {
                    windows::Win32::UI::WindowsAndMessaging::GetSystemMetrics(
                        windows::Win32::UI::WindowsAndMessaging::SM_CXSCREEN,
                    ) as f32
                        / scale
                };
                let screen_h = unsafe {
                    windows::Win32::UI::WindowsAndMessaging::GetSystemMetrics(
                        windows::Win32::UI::WindowsAndMessaging::SM_CYSCREEN,
                    ) as f32
                        / scale
                };

                let px = (lx - popup_w).max(0.0).min(screen_w - popup_w);
                let py = (ly - popup_h).max(0.0).min(screen_h - popup_h);

                let _ = popup_tx.send(PopupRequest {
                    x: px,
                    y: py,
                    w: popup_w,
                    h: popup_h,
                    dark,
                });

                // Wake up the egui main loop so update() runs even if window is hidden
                ctx.request_repaint();
            }
        }
    });

    TrayItems {
        _tray: tray,
        popup_rx,
    }
}

pub fn show_window_native_pub() {
    show_window_native();
}

fn show_window_native() {
    unsafe {
        use windows::Win32::UI::WindowsAndMessaging::{
            BringWindowToTop, FindWindowW, GWL_EXSTYLE, GetWindowLongW, SW_HIDE, SW_RESTORE,
            SetForegroundWindow, SetWindowLongW, ShowWindow, WS_EX_TOOLWINDOW,
        };
        use windows::core::w;
        let hwnd = match FindWindowW(None, w!("DisplayWarp")) {
            Ok(h) if !h.0.is_null() => h,
            _ => return,
        };
        let _ = ShowWindow(hwnd, SW_HIDE);
        let ex = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;
        SetWindowLongW(hwnd, GWL_EXSTYLE, (ex & !WS_EX_TOOLWINDOW.0) as i32);
        let _ = ShowWindow(hwnd, SW_RESTORE);
        let _ = BringWindowToTop(hwnd);
        let _ = SetForegroundWindow(hwnd);
    }
}
