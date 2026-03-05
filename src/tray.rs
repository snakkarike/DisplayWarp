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
        let show_item = MenuItem::new("Show", true, None);
        let quit_item = MenuItem::new("Quit", true, None);
        let menu = Menu::new();
        let _ = menu.append(&show_item);
        let _ = menu.append(&PredefinedMenuItem::separator());

        let mut next_state = TrayState::default();

        let warp_sub = Submenu::new("Warp Profiles", true);
        if profiles.is_empty() {
            let empty = MenuItem::new("(No profiles)", false, None);
            let _ = warp_sub.append(&empty);
        } else {
            for p in profiles {
                let item = MenuItem::new(format!("▶  {}", p.name), true, None);
                next_state.warp_ids.push(item.id().clone());
                let _ = warp_sub.append(&item);
            }
        }
        let _ = menu.append(&warp_sub);

        let disp_sub = Submenu::new("Display Profiles", true);
        if layouts.is_empty() {
            let empty = MenuItem::new("(No layouts)", false, None);
            let _ = disp_sub.append(&empty);
        } else {
            for l in layouts {
                let item = MenuItem::new(format!("🖥  {}", l.name), true, None);
                next_state.display_ids.push(item.id().clone());
                let _ = disp_sub.append(&item);
            }
        }
        let _ = menu.append(&disp_sub);

        let _ = menu.append(&PredefinedMenuItem::separator());
        let _ = menu.append(&quit_item);

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
    let show_item = MenuItem::new("Show", true, None);
    let quit_item = MenuItem::new("Quit", true, None);
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

    let warp_sub = Submenu::new("Warp Profiles", true);
    if profiles_snapshot.is_empty() {
        let empty = MenuItem::new("(No profiles)", false, None);
        let _ = warp_sub.append(&empty);
    } else {
        for p in &profiles_snapshot {
            let item = MenuItem::new(format!("▶  {}", p.name), true, None);
            state.warp_ids.push(item.id().clone());
            let _ = warp_sub.append(&item);
        }
    }
    let _ = menu.append(&warp_sub);

    let disp_sub = Submenu::new("Display Profiles", true);
    if layouts_snapshot.is_empty() {
        let empty = MenuItem::new("(No layouts)", false, None);
        let _ = disp_sub.append(&empty);
    } else {
        for l in &layouts_snapshot {
            let item = MenuItem::new(format!("🖥  {}", l.name), true, None);
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
