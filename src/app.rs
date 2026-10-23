use eframe::egui;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::Graphics::Gdi::{MONITOR_DEFAULTTONEAREST, MonitorFromWindow};
use windows::Win32::UI::Shell::{FOLDERID_Documents, KF_FLAG_DEFAULT, SHGetKnownFolderPath};

use crate::models::{AppProfile, MonitorInfo, SavedData};
use crate::monitor::get_all_monitors;
use crate::window::{
    ProcessEntry, find_window_by_process_name, list_visible_windows, move_window_once,
    wait_for_window, wait_for_window_by_name,
};

// ─── Application State ───────────────────────────────────────────────────────

#[derive(PartialEq, Clone, Copy)]
pub enum AppTab {
    Warp,
    Display,
    Shortcuts,
    Log,
    Settings,
}

#[derive(PartialEq, Clone, Copy, Debug, Default)]
pub enum ShortcutSortMode {
    #[default]
    Default,
    NameAsc,
    NameDesc,
    CategoryAsc,
    CategoryDesc,
}

pub struct WindowManagerApp {
    pub monitors: Vec<MonitorInfo>,
    pub display_targets: Vec<MonitorInfo>,
    /// Shared so the background watcher can read profiles.
    pub data: Arc<parking_lot::Mutex<SavedData>>,
    // ── Navigation ──
    pub current_tab: AppTab,
    // ── New profile form state ──
    pub new_profile_name: String,
    pub new_profile_exe: Option<std::path::PathBuf>,
    pub selected_mon_idx: usize,
    pub new_profile_window_process: String,
    pub new_profile_launch_args: String,
    pub new_profile_window_title: String,
    // ── Edit profile form state ──
    pub editing_profile_idx: Option<usize>,
    pub edit_profile_name: String,
    pub edit_profile_exe: Option<std::path::PathBuf>,
    pub edit_profile_mon_idx: usize,
    pub edit_profile_window_process: String,
    pub edit_profile_launch_args: String,
    pub edit_profile_window_title: String,
    // ── Live-process mover state ──
    pub live_processes: Vec<ProcessEntry>,
    pub selected_live_process_idx: usize,
    pub live_move_mon_idx: usize,
    // ── Display Profile state ──
    pub new_display_profile_name: String,
    pub dragging_monitor_idx: Option<usize>,
    pub selected_display_idx: Option<usize>,
    pub drag_start_pos: Option<egui::Pos2>,
    pub original_monitor_rect: Option<windows::Win32::Foundation::RECT>,

    // ── Desktop Shortcuts layout state ──
    pub new_shortcut_layout_name: String,
    pub shortcut_search_query: String,
    pub shortcut_monitor_filter: Option<usize>,
    pub shortcut_category_filter: Option<String>,
    pub shortcut_sort_mode: ShortcutSortMode,
    pub viewing_shortcut_layout_idx: Option<usize>,
    pub managing_categories: bool,
    pub new_category_name: String,
    pub new_category_color: String,
    pub live_desktop_shortcuts: Vec<crate::desktop_icons::DesktopIconPos>,

    // ── Persistent monitor watcher ──
    pub watcher_running: Arc<AtomicBool>,
    // ── System tray ──
    pub tray: Option<crate::tray::TrayItems>,
    // ── Logo texture ──
    pub logo_texture: Option<eframe::egui::TextureHandle>,
    pub logo_texture_white: Option<eframe::egui::TextureHandle>,
    // ── Audio state ──
    pub audio_devices: Vec<crate::audio::AudioDeviceInfo>,
    pub new_profile_audio_device_idx: usize,
    pub edit_profile_audio_device_idx: usize,
    // ── Shared ──
    pub status_message: Arc<parking_lot::Mutex<String>>,
    pub status_log: Arc<parking_lot::Mutex<Vec<String>>>,
    // ── Theme ──
    pub dark_mode: bool,
    pub theme: crate::ui::theme::Theme,
    // ── Auto-Updater ──
    pub updater: crate::updater::UpdateManager,
    // ── Startup State ──
    pub first_frame_hidden: bool,
}

impl Default for WindowManagerApp {
    fn default() -> Self {
        let data = Arc::new(parking_lot::Mutex::new(SavedData::default()));
        let watcher_running = Arc::new(AtomicBool::new(true));

        let mut app = Self {
            monitors: vec![],
            display_targets: vec![],
            data: Arc::clone(&data),
            current_tab: AppTab::Warp,
            new_profile_name: String::new(),
            new_profile_exe: None,
            selected_mon_idx: 0,
            new_profile_window_process: String::new(),
            new_profile_launch_args: String::new(),
            new_profile_window_title: String::new(),
            editing_profile_idx: None,
            edit_profile_name: String::new(),
            edit_profile_exe: None,
            edit_profile_mon_idx: 0,
            edit_profile_window_process: String::new(),
            edit_profile_launch_args: String::new(),
            edit_profile_window_title: String::new(),
            live_processes: vec![],
            selected_live_process_idx: 0,
            live_move_mon_idx: 0,
            new_display_profile_name: String::new(),
            dragging_monitor_idx: None,
            selected_display_idx: None,
            drag_start_pos: None,
            original_monitor_rect: None,

            new_shortcut_layout_name: String::new(),
            shortcut_search_query: String::new(),
            shortcut_monitor_filter: None,
            shortcut_category_filter: None,
            shortcut_sort_mode: ShortcutSortMode::Default,
            viewing_shortcut_layout_idx: None,
            managing_categories: false,
            new_category_name: String::new(),
            new_category_color: String::from("#10B981"),
            live_desktop_shortcuts: vec![],

            watcher_running: Arc::clone(&watcher_running),
            tray: None,

            logo_texture: None,
            logo_texture_white: None,
            audio_devices: vec![],
            new_profile_audio_device_idx: 0,
            edit_profile_audio_device_idx: 0,
            status_message: Arc::new(parking_lot::Mutex::new(String::from("Ready."))),
            status_log: Arc::new(parking_lot::Mutex::new(vec!["Ready.".to_string()])),
            dark_mode: true,
            theme: crate::ui::theme::Theme::new(true),
            updater: crate::updater::UpdateManager::new(),
            first_frame_hidden: false,
        };
        app.refresh_monitors();
        app.refresh_audio_devices();
        app.refresh_live_processes();
        app.refresh_desktop_shortcuts();
        app.load_data();

        if app.data.lock().check_updates_on_startup {
            app.updater.check_for_updates();
        }

        // Start the background watcher thread.
        Self::start_watcher(Arc::clone(&data), Arc::clone(&watcher_running));

        app
    }
}

// ─── Core Logic ──────────────────────────────────────────────────────────────

impl WindowManagerApp {
    /// Set the status message and append to the log history.
    pub fn push_status(
        status: &Arc<parking_lot::Mutex<String>>,
        log: &Arc<parking_lot::Mutex<Vec<String>>>,
        msg: impl Into<String>,
    ) {
        let msg = msg.into();
        *status.lock() = msg.clone();
        let mut l = log.lock();
        // Prepend a simple timestamp
        let ts = Self::local_time_str();
        l.push(format!("[{ts}] {msg}"));
        if l.len() > 200 {
            l.remove(0);
        }

        // Removed native Windows Toast Notification logic as requested by user.
    }

    /// Returns current local time as HH:MM:SS string for log timestamps.
    fn local_time_str() -> String {
        let secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        // UTC seconds of day — close enough for log display
        let s = secs % 86400;
        let h = s / 3600;
        let m = (s % 3600) / 60;
        let sec = s % 60;
        format!("{h:02}:{m:02}:{sec:02}")
    }

    pub fn refresh_monitors(&mut self) {
        self.monitors = get_all_monitors();
        self.display_targets = crate::monitor::get_all_display_targets();
        let max = self.monitors.len().saturating_sub(1);
        self.selected_mon_idx = self.selected_mon_idx.min(max);
        self.edit_profile_mon_idx = self.edit_profile_mon_idx.min(max);
        self.live_move_mon_idx = self.live_move_mon_idx.min(max);
    }

    pub fn refresh_live_processes(&mut self) {
        self.live_processes = list_visible_windows();
        self.selected_live_process_idx = 0;
    }

    pub fn refresh_audio_devices(&mut self) {
        if let Ok(devices) = crate::audio::get_audio_output_devices() {
            self.audio_devices = devices;
        }
    }

    pub fn get_config_base_dir() -> std::path::PathBuf {
        let exe_dir = std::env::current_exe()
            .unwrap_or_else(|_| std::path::PathBuf::from("."))
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .to_path_buf();

        let mut documents_path = None;
        #[cfg(windows)]
        {
            unsafe {
                if let Ok(path) = SHGetKnownFolderPath(&FOLDERID_Documents, KF_FLAG_DEFAULT, None) {
                    if let Ok(path_str) = path.to_string() {
                        documents_path = Some(std::path::PathBuf::from(path_str));
                    }
                    windows::Win32::System::Com::CoTaskMemFree(Some(path.as_ptr() as *mut _));
                }
            }
        }

        let base_dir = documents_path.unwrap_or_else(|| {
            // Fallback to user home if Documents not found via shell API
            #[cfg(windows)]
            {
                if let Ok(profile) = std::env::var("USERPROFILE") {
                    let p = std::path::PathBuf::from(profile).join("Documents");
                    if p.exists() {
                        return p;
                    }
                }
            }
            exe_dir.clone()
        });

        base_dir.join("DisplayWarp")
    }

    pub fn get_config_path() -> std::path::PathBuf {
        let config_base = Self::get_config_base_dir();
        if !config_base.exists() {
            let _ = std::fs::create_dir_all(&config_base);
        }

        let exe_dir = std::env::current_exe()
            .unwrap_or_else(|_| std::path::PathBuf::from("."))
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .to_path_buf();

        // Migration: Check for manual override in config_location.txt
        let legacy_txt = exe_dir.join("config_location.txt");
        let new_txt = config_base.join("config_location.txt");

        if legacy_txt.exists() && !new_txt.exists() {
            let _ = std::fs::copy(&legacy_txt, &new_txt);
        }

        if let Ok(content) = std::fs::read_to_string(&new_txt) {
            let path = std::path::PathBuf::from(content.trim());
            if path.is_dir() {
                return path.join("monitor_config.json");
            }
        }

        config_base.join("monitor_config.json")
    }

    pub fn load_data(&mut self) {
        let config_path = Self::get_config_path();

        // Migration: If new config doesn't exist, but legacy config in exe dir does, migration!
        if !config_path.exists() {
            let exe_dir = std::env::current_exe()
                .unwrap_or_else(|_| std::path::PathBuf::from("."))
                .parent()
                .unwrap_or_else(|| std::path::Path::new("."))
                .to_path_buf();
            let legacy_path = exe_dir.join("monitor_config.json");

            if legacy_path.exists() {
                if let Ok(bytes) = std::fs::read(&legacy_path) {
                    let _ = std::fs::write(&config_path, bytes);
                    // Optional: remove legacy? Better to keep as backup for now.
                }
            }
        }

        if let Ok(bytes) = std::fs::read(&config_path) {
            if let Ok(decoded) = serde_json::from_slice::<SavedData>(&bytes) {
                *self.data.lock() = decoded;
            }
        }
    }

    pub fn save_data(&self) {
        let data = self.data.lock();
        if let Ok(json) = serde_json::to_string_pretty(&*data) {
            let _ = std::fs::write(Self::get_config_path(), json);
        }
    }

    pub fn refresh_tray(&self) {
        if let Some(t) = &self.tray {
            let d = self.data.lock();
            t.refresh_menu(&d.profiles, &d.display_profiles, &d.shortcut_layouts);
        }
    }

    pub fn refresh_desktop_shortcuts(&mut self) {
        match crate::desktop_icons::capture_desktop_icons() {
            Ok(shortcuts) => {
                let count = shortcuts.len();
                self.live_desktop_shortcuts = shortcuts;
                Self::push_status(
                    &self.status_message,
                    &self.status_log,
                    format!("🔄 Detected {} desktop shortcuts.", count),
                );
            }
            Err(e) => {
                Self::push_status(
                    &self.status_message,
                    &self.status_log,
                    format!("⚠️ Failed to detect desktop shortcuts: {:?}", e),
                );
            }
        }
    }

    pub fn save_shortcut_layout(&mut self, name: &str) {
        let name = name.trim();
        if name.is_empty() {
            return;
        }
        let mut icons = match crate::desktop_icons::capture_desktop_icons() {
            Ok(icons) if !icons.is_empty() => icons,
            _ if !self.live_desktop_shortcuts.is_empty() => self.live_desktop_shortcuts.clone(),
            Ok(icons) => icons,
            Err(e) => {
                Self::push_status(
                    &self.status_message,
                    &self.status_log,
                    format!("⚠️ Failed to capture desktop shortcuts: {:?}", e),
                );
                return;
            }
        };
        {
            let data = self.data.lock();
            for icon in &mut icons {
                icon.category_id = data.shortcut_category_assignments.get(&icon.name).cloned();
            }
        }
        let count = icons.len();
        let timestamp = crate::desktop_icons::current_timestamp_string();
        let layout = crate::desktop_icons::DesktopIconLayout {
            name: name.to_string(),
            created_at: timestamp,
            icons,
        };

        {
            let mut data = self.data.lock();
            if let Some(pos) = data
                .shortcut_layouts
                .iter()
                .position(|l| l.name.eq_ignore_ascii_case(name))
            {
                data.shortcut_layouts[pos] = layout;
            } else {
                data.shortcut_layouts.push(layout);
            }
        }
        self.save_data();
        self.refresh_tray();
        Self::push_status(
            &self.status_message,
            &self.status_log,
            format!("✅ Saved desktop shortcut layout '{}' ({} shortcuts).", name, count),
        );
    }

    pub fn restore_shortcut_layout(&mut self, idx: usize) {
        let layout = {
            let data = self.data.lock();
            data.shortcut_layouts.get(idx).cloned()
        };
        if let Some(layout) = layout {
            {
                let mut data = self.data.lock();
                for icon in &layout.icons {
                    if let Some(cat_id) = &icon.category_id {
                        data.shortcut_category_assignments.insert(icon.name.clone(), cat_id.clone());
                    }
                }
            }
            self.save_data();
            match crate::desktop_icons::restore_desktop_icons(&layout.icons) {
                Ok(restored) => {
                    Self::push_status(
                        &self.status_message,
                        &self.status_log,
                        format!(
                            "✅ Restored {}/{} desktop shortcuts from '{}'.",
                            restored,
                            layout.icons.len(),
                            layout.name
                        ),
                    );
                    self.refresh_desktop_shortcuts();
                }
                Err(e) => {
                    Self::push_status(
                        &self.status_message,
                        &self.status_log,
                        format!("⚠️ Failed to restore layout '{}': {:?}", layout.name, e),
                    );
                }
            }
        }
    }

    pub fn delete_shortcut_layout(&mut self, idx: usize) {
        let removed = {
            let mut data = self.data.lock();
            if idx < data.shortcut_layouts.len() {
                Some(data.shortcut_layouts.remove(idx))
            } else {
                None
            }
        };
        if let Some(layout) = removed {
            self.save_data();
            self.refresh_tray();
            Self::push_status(
                &self.status_message,
                &self.status_log,
                format!("🗑 Deleted desktop shortcut layout '{}'.", layout.name),
            );
        }
    }

    pub fn get_shortcut_category(&self, shortcut_name: &str) -> Option<crate::models::ShortcutCategory> {
        let data = self.data.lock();
        let cat_id = data.shortcut_category_assignments.get(shortcut_name)?;
        data.shortcut_categories.iter().find(|c| &c.id == cat_id).cloned()
    }

    pub fn set_shortcut_category(&mut self, shortcut_name: &str, category_id: Option<String>) {
        {
            let mut data = self.data.lock();
            if let Some(id) = category_id {
                data.shortcut_category_assignments.insert(shortcut_name.to_string(), id);
            } else {
                data.shortcut_category_assignments.remove(shortcut_name);
            }
        }
        self.save_data();
    }

    pub fn add_shortcut_category(&mut self, name: &str, color: &str) -> bool {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return false;
        }
        let cat = crate::models::ShortcutCategory::new(trimmed, color);
        {
            let mut data = self.data.lock();
            if data.shortcut_categories.iter().any(|c| c.name.eq_ignore_ascii_case(trimmed)) {
                return false;
            }
            data.shortcut_categories.push(cat);
        }
        self.save_data();
        true
    }

    pub fn delete_shortcut_category(&mut self, cat_id: &str) {
        {
            let mut data = self.data.lock();
            data.shortcut_categories.retain(|c| c.id != cat_id);
            data.shortcut_category_assignments.retain(|_, v| v != cat_id);
        }
        self.save_data();
    }

    pub fn load_suggested_categories(&mut self) {
        let suggested = vec![
            crate::models::ShortcutCategory::new("Games", "#10B981"),
            crate::models::ShortcutCategory::new("Work", "#3B82F6"),
            crate::models::ShortcutCategory::new("Utilities", "#8B5CF6"),
            crate::models::ShortcutCategory::new("Media", "#F59E0B"),
            crate::models::ShortcutCategory::new("System", "#64748B"),
        ];
        {
            let mut data = self.data.lock();
            for s in suggested {
                if !data.shortcut_categories.iter().any(|c| c.name.eq_ignore_ascii_case(&s.name)) {
                    data.shortcut_categories.push(s);
                }
            }
        }
        self.save_data();
    }

    pub fn get_auto_launch() -> auto_launch::AutoLaunch {
        let app_path =
            std::env::current_exe().unwrap_or_else(|_| std::path::PathBuf::from("DisplayWarp.exe"));
        auto_launch::AutoLaunchBuilder::new()
            .set_app_name("DisplayWarp")
            .set_app_path(&app_path.to_string_lossy())
            .build()
            .unwrap()
    }

    /// Find the live rect for a monitor by device name.
    pub fn find_monitor_rect(monitors: &[MonitorInfo], device_name: &str) -> Option<RECT> {
        monitors
            .iter()
            .find(|m| m.device_name == device_name)
            .map(|m| m.rect)
    }

    // ─── Background watcher ──────────────────────────────────────────────

    fn start_watcher(data: Arc<parking_lot::Mutex<SavedData>>, running: Arc<AtomicBool>) {
        std::thread::spawn(move || {
            while running.load(Ordering::Relaxed) {
                // Dynamically grab the user interval and enabled state
                let (sleep_duration, enabled) = {
                    let d = data.lock();
                    (d.watcher_interval_secs, d.watcher_enabled)
                };

                std::thread::sleep(std::time::Duration::from_secs(sleep_duration.max(1)));
                if !running.load(Ordering::Relaxed) {
                    break;
                }

                if !enabled {
                    continue;
                }

                let profiles: Vec<AppProfile> = { data.lock().profiles.clone() };
                let monitors = get_all_monitors();

                for profile in &profiles {
                    if !profile.persistent_monitor {
                        continue;
                    }
                    let proc_name = match &profile.window_process_name {
                        Some(name) if !name.is_empty() => name.to_lowercase(),
                        _ => continue,
                    };

                    // find_window_by_process_name returns Option<HWND> — fine for watcher
                    let hwnd = match find_window_by_process_name(&proc_name) {
                        Some(h) => h,
                        None => continue,
                    };

                    let target_rect =
                        Self::find_monitor_rect(&monitors, &profile.target_monitor_name).or_else(
                            || {
                                profile.target_monitor_rect.as_ref().map(|r| RECT {
                                    left: r.left,
                                    top: r.top,
                                    right: r.right,
                                    bottom: r.bottom,
                                })
                            },
                        );
                    let target_rect = match target_rect {
                        Some(r) => r,
                        None => continue,
                    };

                    let target_mon = unsafe {
                        use windows::Win32::Foundation::POINT;
                        use windows::Win32::Graphics::Gdi::MonitorFromPoint;
                        let w = target_rect.right - target_rect.left;
                        let h = target_rect.bottom - target_rect.top;
                        MonitorFromPoint(
                            POINT {
                                x: target_rect.left + w / 2,
                                y: target_rect.top + h / 2,
                            },
                            MONITOR_DEFAULTTONEAREST,
                        )
                    };
                    let current_mon = unsafe { MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST) };

                    if current_mon != target_mon {
                        move_window_once(hwnd, target_rect);
                    }
                }
            }
        });
    }

    /// Extracted logic to apply a saved display layout (used by tray menu and UI).
    pub fn apply_display_layout(
        layout: &crate::models::SavedDisplayLayout,
        status_message: Arc<parking_lot::Mutex<String>>,
        status_log: Arc<parking_lot::Mutex<Vec<String>>>,
    ) {
        crate::monitor::restore_monitor_layout(&layout.monitors);
        Self::push_status(
            &status_message,
            &status_log,
            format!("✅ Applied '{}'", layout.name),
        );
    }

    // ─── Profile launching ───────────────────────────────────────────────

    pub fn launch_profile(
        profile: &AppProfile,
        status: Arc<parking_lot::Mutex<String>>,
        log: Arc<parking_lot::Mutex<Vec<String>>>,
    ) {
        let exe = profile.exe_path.clone();
        let device_name = profile.target_monitor_name.clone();
        let window_process_name = profile.window_process_name.clone();
        let audio_device_id = profile.target_audio_device_id.clone();
        let launch_args = profile.launch_args.clone();
        let _window_title_match = profile.window_title_match.clone();

        let live_monitors = get_all_monitors();
        let target_rect = Self::find_monitor_rect(&live_monitors, &device_name).or_else(|| {
            profile.target_monitor_rect.as_ref().map(|r| RECT {
                left: r.left,
                top: r.top,
                right: r.right,
                bottom: r.bottom,
            })
        });
        let target_rect = match target_rect {
            Some(r) => r,
            None => {
                Self::push_status(
                    &status,
                    &log,
                    format!("❌ Monitor '{}' not found.", device_name),
                );
                return;
            }
        };

        let cwd = exe
            .parent()
            .unwrap_or(std::path::Path::new("."))
            .to_path_buf();
        let mut cmd = std::process::Command::new(&exe);
        cmd.current_dir(&cwd);
        if let Some(args_str) = launch_args {
            if !args_str.trim().is_empty() {
                for arg in args_str.split_whitespace() {
                    cmd.arg(arg);
                }
            }
        }
        let child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => {
                Self::push_status(&status, &log, format!("❌ Failed to launch: {e}"));
                return;
            }
        };
        let pid = child.id();
        let exe_name = exe
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "app".to_string());

        Self::push_status(&status, &log, format!("🚀 Launched {exe_name} (PID {pid})"));

        std::thread::spawn(move || {
            unsafe {
                let _ = windows::Win32::System::Com::CoInitializeEx(
                    None,
                    windows::Win32::System::Com::COINIT_APARTMENTTHREADED,
                );
            }

            // ── Audio ──────────────────────────────────────────────────────
            if let Some(ref audio_id) = audio_device_id {
                match crate::audio::set_default_audio_device(audio_id) {
                    Ok(_) => Self::push_status(&status, &log, "🔊 Audio device switched."),
                    Err(e) => {
                        Self::push_status(&status, &log, format!("⚠️ Audio switch failed: {e}"))
                    }
                }
            }

            // ── Window detection ───────────────────────────────────────────
            let target_w = target_rect.right - target_rect.left;
            let target_h = target_rect.bottom - target_rect.top;

            let found = if let Some(proc_name) = window_process_name.filter(|s| !s.is_empty()) {
                Self::push_status(
                    &status,
                    &log,
                    format!("⏳ Waiting for '{proc_name}' window…"),
                );
                wait_for_window_by_name(&proc_name, 30_000)
            } else {
                Self::push_status(&status, &log, format!("⏳ Waiting for PID {pid} window…"));
                wait_for_window(pid, 15_000)
            };

            // ── Move ───────────────────────────────────────────────────────
            match found {
                Some(f) => {
                    Self::push_status(
                        &status,
                        &log,
                        format!(
                            "🪟 Window found ({}×{}) after {:.1}s — moving to {}×{} display…",
                            f.width,
                            f.height,
                            f.elapsed_ms as f32 / 1000.0,
                            target_w,
                            target_h,
                        ),
                    );
                    // Phase 1: aggressive initial placement (~6 s). Runs inline so
                    // we know the window is on the right monitor before reporting done.
                    move_window_once(f.hwnd, target_rect);
                    Self::push_status(&status, &log, "✅ Done — window on target monitor.");
                    // Phase 2: silent 45-second keep-alive watcher in background.
                    // Does not block the status log.
                    let hwnd_raw = f.hwnd.0 as isize;
                    std::thread::spawn(move || {
                        crate::window::watch_window_on_monitor(
                            windows::Win32::Foundation::HWND(hwnd_raw as *mut _),
                            target_rect,
                            45,
                        );
                    });
                }
                None => {
                    Self::push_status(
                        &status,
                        &log,
                        "⚠️ Window not found within timeout. App may still have launched normally.",
                    );
                }
            }

            unsafe {
                windows::Win32::System::Com::CoUninitialize();
            }
        });
    }

    pub fn move_live_window(
        hwnd: HWND,
        target_rect: RECT,
        status: Arc<parking_lot::Mutex<String>>,
        log: Arc<parking_lot::Mutex<Vec<String>>>,
    ) {
        use windows::Win32::UI::WindowsAndMessaging::IsWindow;
        if unsafe { !IsWindow(Some(hwnd)).as_bool() } {
            Self::push_status(
                &status,
                &log,
                "❌ Window no longer exists (it may have been closed).",
            );
            return;
        }
        let hwnd_raw = hwnd.0 as isize;
        std::thread::spawn(move || {
            let hwnd = HWND(hwnd_raw as *mut _);
            move_window_once(hwnd, target_rect);
            Self::push_status(&status, &log, "✅ Window moved to target monitor.");
        });
    }
}
