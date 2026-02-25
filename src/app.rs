use std::sync::Arc;
use windows::Win32::Foundation::RECT;

use crate::models::{AppProfile, MonitorInfo, SavedData, SavedMonitorPos};
use crate::monitor::{get_all_monitors, restore_monitor_layout, switch_primary_to};
use crate::window::{move_to_monitor, wait_for_pid_exit, wait_for_window, wait_for_window_by_name};

// ─── Application State ───────────────────────────────────────────────────────

pub struct WindowManagerApp {
    pub monitors: Vec<MonitorInfo>,
    pub data: SavedData,
    // ── New profile form state ──
    pub new_profile_exe: Option<std::path::PathBuf>,
    pub selected_mon_idx: usize,
    pub new_profile_window_process: String,
    pub new_profile_force_primary: bool,
    // ── Edit profile form state ──
    pub editing_profile_idx: Option<usize>,
    pub edit_profile_exe: Option<std::path::PathBuf>,
    pub edit_profile_mon_idx: usize,
    pub edit_profile_window_process: String,
    pub edit_profile_force_primary: bool,
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
            new_profile_force_primary: false,
            editing_profile_idx: None,
            edit_profile_exe: None,
            edit_profile_mon_idx: 0,
            edit_profile_window_process: String::new(),
            edit_profile_force_primary: false,
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

    /// Launch a profile.
    /// - If force_primary: change the primary monitor first, launch, then restore when process exits.
    /// - Otherwise: launch and use the window-moving strategy.
    pub fn launch_profile(profile: &AppProfile, status: Arc<parking_lot::Mutex<String>>) {
        let exe = profile.exe_path.clone();
        let device_name = profile.target_monitor_name.clone();
        let window_process_name = profile.window_process_name.clone();
        let force_primary = profile.force_primary;

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

        if force_primary {
            // ── Force-Primary path ───────────────────────────────────────────
            // 1. Snapshot current layout so we can restore it
            let snapshot = live_monitors
                .iter()
                .map(|m| SavedMonitorPos {
                    device_name: m.device_name.clone(),
                    rect: m.rect,
                })
                .collect::<Vec<_>>();

            // 2. Switch primary to target monitor
            *status.lock() = format!("⏳ Switching primary to {}...", device_name);
            if !switch_primary_to(&device_name, &live_monitors) {
                *status.lock() = "❌ Failed to switch primary monitor.".into();
                return;
            }
            // Give Windows time to settle
            std::thread::sleep(std::time::Duration::from_millis(1500));

            // 3. Launch the process
            let cwd = exe.parent().unwrap().to_path_buf();
            let child = match std::process::Command::new(&exe).current_dir(&cwd).spawn() {
                Ok(c) => c,
                Err(e) => {
                    *status.lock() = format!("❌ Failed to launch: {e}");
                    restore_monitor_layout(&snapshot);
                    return;
                }
            };
            let pid = child.id();
            *status.lock() =
                format!("⏳ Game launched (PID {pid}) — will restore monitors on exit.");

            // 4. Background thread: wait for process exit, then restore
            std::thread::spawn(move || {
                wait_for_pid_exit(pid);
                restore_monitor_layout(&snapshot);
                *status.lock() = "✅ Game exited. Monitor layout restored.".into();
            });
        } else {
            // ── Window-moving path ───────────────────────────────────────────
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
                            "⚠️ Window not found within timeout (app may still work normally)."
                                .into();
                    }
                }
            });
        }
    }
}
