use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use windows::Win32::Foundation::RECT;

// ─── Data Models ─────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AppProfile {
    pub name: String,
    pub exe_path: PathBuf,
    /// Windows device name of the target monitor, e.g. "\\.\ DISPLAY2"
    pub target_monitor_name: String,
    /// Cached rect of the target monitor at save time (used as fallback)
    #[serde(default)]
    pub target_monitor_rect: Option<SerializableRect>,
    /// Optional: if the launcher spawns a different process (e.g. a game),
    /// set this to that process' exe name, e.g. "Diablo IV.exe".
    /// Leave None to track the launched process itself.
    #[serde(default)]
    pub window_process_name: Option<String>,
    /// For exclusive fullscreen games: temporarily make the target monitor
    /// primary before launching, then restore after the process exits.
    #[serde(default)]
    pub force_primary: bool,
    /// When true, the background watcher will continuously enforce that this
    /// app's window stays on the target monitor.
    #[serde(default)]
    pub persistent_monitor: bool,
    /// Optional: target audio output device ID. If set, switching to this device
    /// will be attempted on launch/move.
    #[serde(default)]
    pub target_audio_device_id: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct SerializableRect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

#[derive(Serialize, Deserialize)]
pub struct SavedData {
    pub profiles: Vec<AppProfile>,
    #[serde(default)]
    pub close_to_tray: bool,
    #[serde(default)]
    pub start_minimized: bool,
    #[serde(default)]
    pub start_on_boot: bool,
    #[serde(default = "default_watcher_interval")]
    pub watcher_interval_secs: u64,
}

impl Default for SavedData {
    fn default() -> Self {
        Self {
            profiles: vec![],
            close_to_tray: false,
            start_minimized: false,
            start_on_boot: false,
            watcher_interval_secs: 3,
        }
    }
}

fn default_watcher_interval() -> u64 {
    3
}

// ─── Runtime State ───────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub struct MonitorInfo {
    pub rect: RECT,
    pub device_name: String,
}

#[derive(Clone)]
#[allow(dead_code)]
pub struct SavedMonitorPos {
    pub device_name: String,
    pub rect: RECT,
}
