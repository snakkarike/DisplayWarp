use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use tray_icon::menu::{Menu, MenuEvent, MenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

/// Render the DisplayWarp SVG icon at 32×32 for the tray.
fn tray_icon() -> Icon {
    let rgba =
        crate::svg_render::svg_to_rgba(include_bytes!("../assets/DisplayWarpIcon.svg"), 32, 32);
    Icon::from_rgba(rgba, 32, 32).expect("failed to create tray icon")
}

pub struct TrayItems {
    pub _tray: TrayIcon, // must stay alive
}

/// Create the system-tray icon and context menu.
/// Spawns a background thread that handles menu events directly via Win32 —
/// completely independent of eframe's event loop.
pub fn create_tray(watcher_running: Arc<AtomicBool>) -> TrayItems {
    let show_item = MenuItem::new("Show", true, None);
    let quit_item = MenuItem::new("Quit", true, None);
    let show_id = show_item.id().clone();
    let quit_id = quit_item.id().clone();

    let menu = Menu::new();
    let _ = menu.append(&show_item);
    let _ = menu.append(&quit_item);

    let icon = tray_icon();

    let tray = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip("DisplayWarp")
        .with_icon(icon)
        .build()
        .expect("failed to build tray icon");

    // Background thread that handles tray events using Win32 directly.
    // Does NOT depend on eframe's update() being called.
    std::thread::spawn(move || {
        let receiver = MenuEvent::receiver();
        loop {
            if let Ok(event) = receiver.recv() {
                let id = event.id().clone();
                if id == quit_id {
                    watcher_running.store(false, Ordering::Relaxed);
                    std::process::exit(0);
                } else if id == show_id {
                    // Show the window directly via Win32 — works even when
                    // eframe's event loop is throttled.
                    show_window_native();
                }
            }
        }
    });

    TrayItems { _tray: tray }
}

/// Show the DisplayWarp window using native Win32 calls.
/// Called from the tray background thread — no eframe dependency.
fn show_window_native() {
    unsafe {
        use windows::Win32::UI::WindowsAndMessaging::{
            BringWindowToTop, FindWindowW, GWL_EXSTYLE, GetWindowLongW, SW_HIDE, SW_RESTORE,
            SetForegroundWindow, SetWindowLongW, ShowWindow, WS_EX_TOOLWINDOW,
        };
        use windows::core::w;

        let hwnd = match FindWindowW(None, w!("Display Warp")) {
            Ok(h) if !h.0.is_null() => h,
            _ => return,
        };

        // 1. Hide briefly to avoid style-change glitch.
        let _ = ShowWindow(hwnd, SW_HIDE);
        // 2. Remove TOOLWINDOW so taskbar button reappears.
        let ex = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;
        SetWindowLongW(hwnd, GWL_EXSTYLE, (ex & !WS_EX_TOOLWINDOW.0) as i32);
        // 3. Restore normally.
        let _ = ShowWindow(hwnd, SW_RESTORE);
        let _ = BringWindowToTop(hwnd);
        let _ = SetForegroundWindow(hwnd);
    }
}
