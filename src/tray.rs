use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use parking_lot::Mutex;
use tray_icon::menu::{Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem, Submenu};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

use crate::models::{AppProfile, SavedData, SavedDisplayLayout};

fn tray_icon() -> Icon {
    let (rgba, w, h) =
        crate::svg_render::png_to_rgba(include_bytes!("../assets/DisplayWarpIcon64.png"));
    Icon::from_rgba(rgba, w, h).expect("failed to create tray icon")
}

/// Detects whether Windows is currently in dark mode.
/// Returns `true` for dark mode, `false` for light mode.
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
        let open_result = RegOpenKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR(subkey.as_ptr()),
            Some(0),
            KEY_READ,
            &mut hkey,
        );
        if open_result.is_err() {
            return false;
        }

        let mut data: u32 = 0;
        let mut data_size: u32 = std::mem::size_of::<u32>() as u32;

        let query_result = RegQueryValueExW(
            hkey,
            PCWSTR(value_name.as_ptr()),
            None,
            None,
            Some(&mut data as *mut u32 as *mut u8),
            Some(&mut data_size),
        );

        let _ = RegCloseKey(hkey);

        if query_result.is_ok() {
            data == 0
        } else {
            false
        }
    }
}

/// Ordinal-based imports from the undocumented uxtheme.dll dark mode API.
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

    /// ordinal 133 — AllowDarkModeForWindow
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

    /// ordinal 135 — SetPreferredAppMode (2 = ForceDark, 1 = AllowDark, 0 = Default)
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

    /// ordinal 136 — FlushMenuThemes
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

/// Apply dark/light mode to the app window title bar and context menus.
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

#[derive(Default, Clone)]
pub struct TrayState {
    pub warp_ids: Vec<MenuId>,
    pub display_ids: Vec<MenuId>,
}

pub struct TrayItems {
    tray: TrayIcon,
    /// Shared with the background event thread — updated on every menu rebuild.
    state: Arc<Mutex<TrayState>>,
}

impl TrayItems {
    pub fn refresh_menu(&self, profiles: &[AppProfile], layouts: &[SavedDisplayLayout]) {
        let dark = is_dark_mode();

        if dark {
            uxtheme::set_preferred_app_mode(2);
        } else {
            uxtheme::set_preferred_app_mode(1);
        }
        uxtheme::flush_menu_themes();
        apply_window_dark_mode(dark);

        let warp_icon = "▶️  ";
        let display_icon = "▶️  ";
        let empty_icon = "📂  ";

        let show_item = MenuItem::new("👁  Show", true, None);
        let quit_item = MenuItem::new("❌  Quit", true, None);
        let menu = Menu::new();
        let _ = menu.append(&show_item);
        let _ = menu.append(&PredefinedMenuItem::separator());

        let mut next_state = TrayState::default();

        let warp_sub = Submenu::new("⚡  Warp Profiles", true);
        if profiles.is_empty() {
            let empty = MenuItem::new(format!("{}(No profiles)", empty_icon), false, None);
            let _ = warp_sub.append(&empty);
        } else {
            for p in profiles {
                let item = MenuItem::new(format!("{}{}", warp_icon, p.name), true, None);
                next_state.warp_ids.push(item.id().clone());
                let _ = warp_sub.append(&item);
            }
        }
        let _ = menu.append(&warp_sub);

        let disp_sub = Submenu::new("🖥  Display Profiles", true);
        if layouts.is_empty() {
            let empty = MenuItem::new(format!("{}(No layouts)", empty_icon), false, None);
            let _ = disp_sub.append(&empty);
        } else {
            for l in layouts {
                let item = MenuItem::new(format!("{}{}", display_icon, l.name), true, None);
                next_state.display_ids.push(item.id().clone());
                let _ = disp_sub.append(&item);
            }
        }
        let _ = menu.append(&disp_sub);

        let _ = menu.append(&PredefinedMenuItem::separator());
        let _ = menu.append(&quit_item);

        let _ = self.tray.set_tooltip(Some("DisplayWarp".to_string()));

        *self.state.lock() = next_state;
        self.tray.set_menu(Some(Box::new(menu)));
    }
}

/// Create the system-tray icon and context menu.
pub fn create_tray(
    watcher_running: Arc<AtomicBool>,
    data: Arc<Mutex<SavedData>>,
    status_message: Arc<Mutex<String>>,
    status_log: Arc<Mutex<Vec<String>>>,
) -> TrayItems {
    let dark = is_dark_mode();

    if dark {
        uxtheme::set_preferred_app_mode(2);
    } else {
        uxtheme::set_preferred_app_mode(1);
    }
    uxtheme::flush_menu_themes();

    let warp_icon = "▶️  ";
    let display_icon = "▶️  ";
    let empty_icon = "📂  ";

    let show_item = MenuItem::new("👁  Show", true, None);
    let quit_item = MenuItem::new("❌  Quit", true, None);
    let show_id = show_item.id().clone();
    let quit_id = quit_item.id().clone();

    let menu = Menu::new();
    let _ = menu.append(&show_item);
    let _ = menu.append(&PredefinedMenuItem::separator());

    let mut state = TrayState::default();

    let (profiles_snapshot, layouts_snapshot) = {
        let d = data.lock();
        (d.profiles.clone(), d.display_profiles.clone())
    };

    let warp_sub = Submenu::new("⚡  Warp Profiles", true);
    if profiles_snapshot.is_empty() {
        let empty = MenuItem::new(format!("{}(No profiles)", empty_icon), false, None);
        let _ = warp_sub.append(&empty);
    } else {
        for p in &profiles_snapshot {
            let item = MenuItem::new(format!("{}{}", warp_icon, p.name), true, None);
            state.warp_ids.push(item.id().clone());
            let _ = warp_sub.append(&item);
        }
    }
    let _ = menu.append(&warp_sub);

    let disp_sub = Submenu::new("🖥  Display Profiles", true);
    if layouts_snapshot.is_empty() {
        let empty = MenuItem::new(format!("{}(No layouts)", empty_icon), false, None);
        let _ = disp_sub.append(&empty);
    } else {
        for l in &layouts_snapshot {
            let item = MenuItem::new(format!("{}{}", display_icon, l.name), true, None);
            state.display_ids.push(item.id().clone());
            let _ = disp_sub.append(&item);
        }
    }
    let _ = menu.append(&disp_sub);

    let _ = menu.append(&PredefinedMenuItem::separator());
    let _ = menu.append(&quit_item);

    let icon = tray_icon();
    let tray = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip("DisplayWarp")
        .with_icon(icon)
        .build()
        .expect("failed to build tray icon");

    let state_arc = Arc::new(Mutex::new(state));
    let state_thread = Arc::clone(&state_arc);

    std::thread::spawn(move || {
        let receiver = MenuEvent::receiver();
        loop {
            if let Ok(event) = receiver.recv() {
                let id = event.id().clone();
                if id == quit_id {
                    watcher_running.store(false, Ordering::Relaxed);
                    std::process::exit(0);
                } else if id == show_id {
                    show_window_native();
                } else {
                    let st = state_thread.lock().clone();

                    // Check Warp Profiles
                    let mut found = false;
                    for (idx, pid) in st.warp_ids.iter().enumerate() {
                        if id == *pid {
                            found = true;
                            let profile = {
                                let d = data.lock();
                                d.profiles.get(idx).cloned()
                            };
                            if let Some(p) = profile {
                                crate::app::WindowManagerApp::launch_profile(
                                    &p,
                                    Arc::clone(&status_message),
                                    Arc::clone(&status_log),
                                );
                            }
                            break;
                        }
                    }
                    if found {
                        continue;
                    }

                    // Check Display Profiles
                    for (idx, pid) in st.display_ids.iter().enumerate() {
                        if id == *pid {
                            let layout = {
                                let d = data.lock();
                                d.display_profiles.get(idx).cloned()
                            };
                            if let Some(l) = layout {
                                crate::app::WindowManagerApp::apply_display_layout(
                                    &l,
                                    Arc::clone(&status_message),
                                    Arc::clone(&status_log),
                                );
                            }
                            break;
                        }
                    }
                }
            }
        }
    });

    TrayItems {
        tray,
        state: state_arc,
    }
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
