use eframe::egui;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::Graphics::Gdi::{MONITOR_DEFAULTTONEAREST, MonitorFromWindow};

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
    Log,
    Settings,
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
    pub markdown_cache: egui_commonmark::CommonMarkCache,
    // ── Theme ──
    pub dark_mode: bool,
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

            watcher_running: Arc::clone(&watcher_running),
            tray: None,

            logo_texture: None,
            logo_texture_white: None,
            audio_devices: vec![],
            new_profile_audio_device_idx: 0,
            edit_profile_audio_device_idx: 0,
            status_message: Arc::new(parking_lot::Mutex::new(String::from("Ready."))),
            status_log: Arc::new(parking_lot::Mutex::new(vec!["Ready.".to_string()])),
            markdown_cache: egui_commonmark::CommonMarkCache::default(),
            dark_mode: true,
            first_frame_hidden: false,
        };
        app.refresh_monitors();
        app.refresh_audio_devices();
        app.refresh_live_processes();
        app.load_data();

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

        // Trigger native Windows Toast Notification for important events
        let should_toast = msg.starts_with("✅")
            || msg.starts_with("❌")
            || msg.starts_with("⚠️")
            || msg.starts_with("🚀")
            || msg.starts_with("🗑")
            || msg.starts_with("📦")
            || msg.starts_with("📁")
            || msg.starts_with("⚙️");

        if should_toast {
            // Strip the emoji prefix for the toast title/text if we want,
            // or just show the whole thing. Let's show the whole thing as text1.
            let toast_msg = msg.clone();
            std::thread::spawn(move || {
                #[cfg(windows)]
                {
                    use winrt_notification::{Duration, Sound, Toast};
                    // Use a generic App ID since we don't have a registered one
                    let _ = Toast::new(Toast::POWERSHELL_APP_ID)
                        .title("DisplayWarp")
                        .text1(&toast_msg)
                        .sound(Some(Sound::SMS))
                        .duration(Duration::Short)
                        .show();
                }
            });
        }
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

    pub fn get_config_path() -> std::path::PathBuf {
        let exe_dir = std::env::current_exe()
            .unwrap_or_else(|_| std::path::PathBuf::from("."))
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .to_path_buf();

        if let Ok(content) = std::fs::read_to_string(exe_dir.join("config_location.txt")) {
            let path = std::path::PathBuf::from(content.trim());
            if path.is_dir() {
                return path.join("monitor_config.json");
            }
        }

        exe_dir.join("monitor_config.json")
    }

    pub fn load_data(&mut self) {
        if let Ok(bytes) = std::fs::read(Self::get_config_path()) {
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
                // Dynamically grab the user interval to sleep
                let sleep_duration = { data.lock().watcher_interval_secs };

                std::thread::sleep(std::time::Duration::from_secs(sleep_duration));
                if !running.load(Ordering::Relaxed) {
                    break;
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
