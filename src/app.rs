use std::sync::Arc;
use windows::Win32::Foundation::{HWND, RECT};

use crate::models::{AppProfile, MonitorInfo, SavedData};
use crate::monitor::get_all_monitors;
use crate::window::{
    ProcessEntry, list_visible_windows, move_to_monitor, wait_for_window, wait_for_window_by_name,
};

// ─── Application State ───────────────────────────────────────────────────────

pub struct WindowManagerApp {
    pub monitors: Vec<MonitorInfo>,
    pub data: SavedData,
    // ── New profile form state ──
    pub new_profile_exe: Option<std::path::PathBuf>,
    pub selected_mon_idx: usize,
    pub new_profile_window_process: String,
    // ── Edit profile form state ──
    pub editing_profile_idx: Option<usize>,
    pub edit_profile_exe: Option<std::path::PathBuf>,
    pub edit_profile_mon_idx: usize,
    pub edit_profile_window_process: String,
    // ── Live-process mover state ──
    pub live_processes: Vec<ProcessEntry>,
    pub selected_live_process_idx: usize,
    pub live_move_mon_idx: usize,
    // ── Shared ──
    pub status_message: Arc<parking_lot::Mutex<String>>,
}

impl Default for WindowManagerApp {
    fn default() -> Self {
        let mut app = Self {
            monitors: vec![],
            data: SavedData::default(),
            new_profile_exe: None,
            selected_mon_idx: 0,
            new_profile_window_process: String::new(),
            editing_profile_idx: None,
            edit_profile_exe: None,
            edit_profile_mon_idx: 0,
            edit_profile_window_process: String::new(),
            live_processes: vec![],
            selected_live_process_idx: 0,
            live_move_mon_idx: 0,
            status_message: Arc::new(parking_lot::Mutex::new(String::from("Ready."))),
        };
        app.refresh_monitors();
        app.load_data();
        app
    }
}

// ─── Core Logic ──────────────────────────────────────────────────────────────

impl WindowManagerApp {
    pub fn refresh_monitors(&mut self) {
        self.monitors = get_all_monitors();
    }

    pub fn refresh_live_processes(&mut self) {
        self.live_processes = list_visible_windows();
        self.selected_live_process_idx = 0;
    }

    pub fn load_data(&mut self) {
        if let Ok(bytes) = std::fs::read("monitor_config.json") {
            if let Ok(decoded) = serde_json::from_slice::<SavedData>(&bytes) {
                self.data = decoded;
            }
        }
    }

    pub fn save_data(&self) {
        if let Ok(json) = serde_json::to_string_pretty(&self.data) {
            let _ = std::fs::write("monitor_config.json", json);
        }
    }

    /// Find the live rect for a monitor by device name.
    pub fn find_monitor_rect(monitors: &[MonitorInfo], device_name: &str) -> Option<RECT> {
        monitors
            .iter()
            .find(|m| m.device_name == device_name)
            .map(|m| m.rect)
    }

    /// Launch a profile using the window-moving strategy.
    pub fn launch_profile(profile: &AppProfile, status: Arc<parking_lot::Mutex<String>>) {
        let exe = profile.exe_path.clone();
        let device_name = profile.target_monitor_name.clone();
        let window_process_name = profile.window_process_name.clone();

        // Resolve target rect.
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
                *status.lock() = format!("❌ Monitor '{}' not found.", device_name);
                return;
            }
        };

        // ── Window-moving path ───────────────────────────────────────────────
        let cwd = exe.parent().unwrap().to_path_buf();
        let child = match std::process::Command::new(&exe).current_dir(&cwd).spawn() {
            Ok(c) => c,
            Err(e) => {
                *status.lock() = format!("❌ Failed to launch: {e}");
                return;
            }
        };
        let pid = child.id();
        std::thread::spawn(move || {
            let hwnd = if let Some(proc_name) = window_process_name.filter(|s| !s.is_empty()) {
                *status.lock() = format!("⏳ Waiting for '{proc_name}' window…");
                wait_for_window_by_name(&proc_name, 30_000)
            } else {
                *status.lock() = format!("⏳ Launched PID {pid}, waiting for window…");
                wait_for_window(pid, 15_000)
            };
            match hwnd {
                Some(h) => {
                    move_to_monitor(h, target_rect);
                    *status.lock() = "✅ Window locked on target monitor.".into();
                }
                None => {
                    *status.lock() =
                        "⚠️ Window not found within timeout (app may still work normally).".into();
                }
            }
        });
    }

    /// Move an already-running window (from the live-process list) to a target monitor rect.
    pub fn move_live_window(
        hwnd: HWND,
        target_rect: RECT,
        status: Arc<parking_lot::Mutex<String>>,
    ) {
        // HWND is not Send, so transmit the raw handle value as isize.
        let hwnd_raw = hwnd.0 as isize;
        std::thread::spawn(move || {
            let hwnd = HWND(hwnd_raw as *mut _);
            move_to_monitor(hwnd, target_rect);
            *status.lock() = "✅ Window moved to target monitor.".into();
        });
    }
}
