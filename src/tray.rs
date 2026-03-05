use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use parking_lot::Mutex;
use tray_icon::menu::{Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

use crate::models::{AppProfile, SavedData};

fn tray_icon() -> Icon {
    let (rgba, w, h) =
        crate::svg_render::png_to_rgba(include_bytes!("../assets/DisplayWarpIcon64.png"));
    Icon::from_rgba(rgba, w, h).expect("failed to create tray icon")
}

pub struct TrayItems {
    tray: TrayIcon,
    /// Shared with the background event thread — updated on every menu rebuild.
    profile_ids: Arc<Mutex<Vec<MenuId>>>,
}

impl TrayItems {
    /// Rebuild the tray context menu from the latest profiles list and update
    /// the shared ID list so the background thread picks up the new IDs.
    pub fn refresh_menu(&self, profiles: &[AppProfile]) {
        let show_item = MenuItem::new("Show", true, None);
        let quit_item = MenuItem::new("Quit", true, None);
        let menu = Menu::new();
        let _ = menu.append(&show_item);

        let mut new_ids = Vec::with_capacity(profiles.len());
        if !profiles.is_empty() {
            let _ = menu.append(&PredefinedMenuItem::separator());
            for p in profiles {
                let item = MenuItem::new(format!("▶  {}", p.name), true, None);
                new_ids.push(item.id().clone());
                let _ = menu.append(&item);
            }
            let _ = menu.append(&PredefinedMenuItem::separator());
        }
        let _ = menu.append(&quit_item);

        // Push new IDs to the shared list BEFORE setting the menu
        *self.profile_ids.lock() = new_ids;
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
    let show_item = MenuItem::new("Show", true, None);
    let quit_item = MenuItem::new("Quit", true, None);
    let show_id = show_item.id().clone();
    let quit_id = quit_item.id().clone();

    // Build initial profile menu from current saved data
    let profiles_snapshot = data.lock().profiles.clone();
    let mut initial_profile_ids: Vec<MenuId> = Vec::with_capacity(profiles_snapshot.len());
    let profile_items: Vec<MenuItem> = profiles_snapshot
        .iter()
        .map(|p| {
            let item = MenuItem::new(format!("▶  {}", p.name), true, None);
            initial_profile_ids.push(item.id().clone());
            item
        })
        .collect();

    let menu = Menu::new();
    let _ = menu.append(&show_item);
    if !profile_items.is_empty() {
        let _ = menu.append(&PredefinedMenuItem::separator());
        for item in &profile_items {
            let _ = menu.append(item);
        }
        let _ = menu.append(&PredefinedMenuItem::separator());
    }
    let _ = menu.append(&quit_item);

    let icon = tray_icon();
    let tray = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip("DisplayWarp")
        .with_icon(icon)
        .build()
        .expect("failed to build tray icon");

    // Shared profile IDs — updated by refresh_menu, read by background thread.
    let profile_ids = Arc::new(Mutex::new(initial_profile_ids));
    let profile_ids_thread = Arc::clone(&profile_ids);

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
                    // Snapshot the current IDs so we don't hold the lock while launching.
                    let ids = profile_ids_thread.lock().clone();
                    for (profile_idx, pid) in ids.iter().enumerate() {
                        if id == *pid {
                            let profile = {
                                let d = data.lock();
                                d.profiles.get(profile_idx).cloned()
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
                }
            }
        }
    });

    TrayItems { tray, profile_ids }
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
