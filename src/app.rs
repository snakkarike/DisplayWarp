use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::Graphics::Gdi::{MONITOR_DEFAULTTONEAREST, MonitorFromWindow};

use crate::models::{AppProfile, MonitorInfo, SavedData};
use crate::monitor::get_all_monitors;
use crate::window::{
    ProcessEntry, find_window_by_process_name, list_visible_windows, move_to_monitor,
    move_window_once, wait_for_window, wait_for_window_by_name,
};

// ─── Application State ───────────────────────────────────────────────────────

pub struct WindowManagerApp {
    pub monitors: Vec<MonitorInfo>,
    /// Shared so the background watcher can read profiles.
    pub data: Arc<parking_lot::Mutex<SavedData>>,
    // ── New profile form state ──
    pub new_profile_name: String,
    pub new_profile_exe: Option<std::path::PathBuf>,
    pub selected_mon_idx: usize,
    pub new_profile_window_process: String,
    // ── Edit profile form state ──
    pub editing_profile_idx: Option<usize>,
    pub edit_profile_name: String,
    pub edit_profile_exe: Option<std::path::PathBuf>,
    pub edit_profile_mon_idx: usize,
    pub edit_profile_window_process: String,
    // ── Live-process mover state ──
    pub live_processes: Vec<ProcessEntry>,
    pub selected_live_process_idx: usize,
    pub live_move_mon_idx: usize,
    // ── Persistent monitor watcher ──
    pub watcher_running: Arc<AtomicBool>,
    // ── System tray ──
    pub tray: Option<crate::tray::TrayItems>,
    // ── Close dialog ──
    pub show_close_dialog: bool,
    // ── Logo texture ──
    pub logo_texture: Option<eframe::egui::TextureHandle>,
    // ── Audio state ──
    pub audio_devices: Vec<crate::audio::AudioDeviceInfo>,
    pub new_profile_audio_device_idx: usize,
    pub edit_profile_audio_device_idx: usize,
    // ── Shared ──
    pub status_message: Arc<parking_lot::Mutex<String>>,
    pub status_log: Arc<parking_lot::Mutex<Vec<String>>>,
    // ── Theme ──
    pub dark_mode: bool,
}

impl Default for WindowManagerApp {
    fn default() -> Self {
        let data = Arc::new(parking_lot::Mutex::new(SavedData::default()));
        let watcher_running = Arc::new(AtomicBool::new(true));

        let mut app = Self {
            monitors: vec![],
            data: Arc::clone(&data),
            new_profile_name: String::new(),
            new_profile_exe: None,
            selected_mon_idx: 0,
            new_profile_window_process: String::new(),
            editing_profile_idx: None,
            edit_profile_name: String::new(),
            edit_profile_exe: None,
            edit_profile_mon_idx: 0,
            edit_profile_window_process: String::new(),
            live_processes: vec![],
            selected_live_process_idx: 0,
            live_move_mon_idx: 0,
            watcher_running: Arc::clone(&watcher_running),
            tray: None,
            show_close_dialog: false,
            logo_texture: None,
            audio_devices: vec![],
            new_profile_audio_device_idx: 0,
            edit_profile_audio_device_idx: 0,
            status_message: Arc::new(parking_lot::Mutex::new(String::from("Ready."))),
            status_log: Arc::new(parking_lot::Mutex::new(vec!["Ready.".to_string()])),
            dark_mode: true,
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
        l.push(msg);
        if l.len() > 200 {
            l.remove(0);
        }
    }

    pub fn refresh_monitors(&mut self) {
        self.monitors = get_all_monitors();
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

    pub fn load_data(&mut self) {
        if let Ok(bytes) = std::fs::read("monitor_config.json") {
            if let Ok(decoded) = serde_json::from_slice::<SavedData>(&bytes) {
                *self.data.lock() = decoded;
            }
        }
    }

    pub fn save_data(&self) {
        let data = self.data.lock();
        if let Ok(json) = serde_json::to_string_pretty(&*data) {
            let _ = std::fs::write("monitor_config.json", json);
        }
    }

    /// Find the live rect for a monitor by device name.
    pub fn find_monitor_rect(monitors: &[MonitorInfo], device_name: &str) -> Option<RECT> {
        monitors
            .iter()
            .find(|m| m.device_name == device_name)
            .map(|m| m.rect.into())
    }

    // ─── Background watcher ──────────────────────────────────────────────

    fn start_watcher(data: Arc<parking_lot::Mutex<SavedData>>, running: Arc<AtomicBool>) {
        std::thread::spawn(move || {
            while running.load(Ordering::Relaxed) {
                std::thread::sleep(std::time::Duration::from_secs(3));
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
        let child = match std::process::Command::new(&exe).current_dir(&cwd).spawn() {
            Ok(c) => c,
            Err(e) => {
                Self::push_status(&status, &log, format!("❌ Failed to launch: {e}"));
                return;
            }
        };
        let pid = child.id();

        std::thread::spawn(move || {
            unsafe {
                let _ = windows::Win32::System::Com::CoInitializeEx(
                    None,
                    windows::Win32::System::Com::COINIT_APARTMENTTHREADED,
                );
            }

            if let Some(audio_id) = &audio_device_id {
                Self::push_status(
                    &status,
                    &log,
                    format!(
                        "🔍 Trying audio ID: {}",
                        &audio_id[..audio_id.len().min(60)]
                    ),
                );
                match crate::audio::set_default_audio_device(audio_id) {
                    Ok(_) => {
                        Self::push_status(&status, &log, "🎵 Audio switched, waiting for window…")
                    }
                    Err(e) => {
                        Self::push_status(&status, &log, format!("⚠️ Audio switch failed: {e}"))
                    }
                }
            }

            let hwnd = if let Some(proc_name) = window_process_name.filter(|s| !s.is_empty()) {
                Self::push_status(
                    &status,
                    &log,
                    format!("⏳ Waiting for '{proc_name}' window…"),
                );
                wait_for_window_by_name(&proc_name, 30_000)
            } else {
                Self::push_status(
                    &status,
                    &log,
                    format!("⏳ Launched PID {pid}, waiting for window…"),
                );
                wait_for_window(pid, 15_000)
            };

            match hwnd {
                Some(h) => {
                    move_to_monitor(h, target_rect);
                    Self::push_status(
                        &status,
                        &log,
                        match &audio_device_id {
                            Some(_) => "✅ Window moved + audio switched.".to_string(),
                            None => "✅ Window locked on target monitor.".to_string(),
                        },
                    );
                }
                None => {
                    Self::push_status(
                        &status,
                        &log,
                        "⚠️ Window not found within timeout (app may still work normally).",
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
