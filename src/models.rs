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
    /// Optional command-line arguments to pass when launching the profile.
    #[serde(default)]
    pub launch_args: Option<String>,
    /// Optional window title wildcard matching, to capture dynamically named windows.
    #[serde(default)]
    pub window_title_match: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct SerializableRect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl SerializableRect {
    pub fn intersects_rect(&self, other: &windows::Win32::Foundation::RECT) -> bool {
        self.left < other.right
            && self.right > other.left
            && self.top < other.bottom
            && self.bottom > other.top
    }

    pub fn to_rect(&self) -> windows::Win32::Foundation::RECT {
        windows::Win32::Foundation::RECT {
            left: self.left,
            top: self.top,
            right: self.right,
            bottom: self.bottom,
        }
    }
}

impl From<windows::Win32::Foundation::RECT> for SerializableRect {
    fn from(r: windows::Win32::Foundation::RECT) -> Self {
        Self {
            left: r.left,
            top: r.top,
            right: r.right,
            bottom: r.bottom,
        }
    }
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
    #[serde(default = "default_true")]
    pub watcher_enabled: bool,
    #[serde(default = "default_watcher_interval")]
    pub watcher_interval_secs: u64,
    #[serde(default)]
    pub display_profiles: Vec<SavedDisplayLayout>,
    #[serde(default)]
    pub shortcut_layouts: Vec<crate::desktop_icons::DesktopIconLayout>,
    #[serde(default)]
    pub shortcut_categories: Vec<ShortcutCategory>,
    #[serde(default)]
    pub shortcut_category_assignments: std::collections::HashMap<String, String>,
    #[serde(default = "default_true")]
    pub check_updates_on_startup: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ShortcutCategory {
    pub id: String,
    pub name: String,
    pub color: String,
}

impl ShortcutCategory {
    pub fn new(name: &str, color: &str) -> Self {
        let id = name
            .trim()
            .to_lowercase()
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
            .collect::<String>();
        let id = if id.is_empty() {
            format!(
                "cat_{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis()
            )
        } else {
            id
        };
        Self {
            id,
            name: name.trim().to_string(),
            color: color.to_string(),
        }
    }

    pub fn color32(&self) -> eframe::egui::Color32 {
        parse_hex_color(&self.color).unwrap_or(eframe::egui::Color32::from_rgb(139, 92, 246))
    }
}

pub const PRESET_CATEGORY_COLORS: &[(&str, &str)] = &[
    ("Emerald", "#10B981"),
    ("Blue", "#3B82F6"),
    ("Purple", "#8B5CF6"),
    ("Amber", "#F59E0B"),
    ("Rose", "#F43F5E"),
    ("Cyan", "#06B6D4"),
    ("Orange", "#F97316"),
    ("Pink", "#EC4899"),
    ("Indigo", "#6366F1"),
    ("Slate", "#64748B"),
];

pub fn parse_hex_color(hex: &str) -> Option<eframe::egui::Color32> {
    let s = hex.trim().trim_start_matches('#');
    if s.len() == 6 {
        let r = u8::from_str_radix(&s[0..2], 16).ok()?;
        let g = u8::from_str_radix(&s[2..4], 16).ok()?;
        let b = u8::from_str_radix(&s[4..6], 16).ok()?;
        Some(eframe::egui::Color32::from_rgb(r, g, b))
    } else {
        None
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SavedDisplayLayout {
    pub name: String,
    pub monitors: Vec<SavedMonitorPos>,
}

impl Default for SavedData {
    fn default() -> Self {
        Self {
            profiles: vec![],
            close_to_tray: false,
            start_minimized: false,
            start_on_boot: false,
            watcher_enabled: true,
            watcher_interval_secs: 3,
            display_profiles: vec![],
            shortcut_layouts: vec![],
            shortcut_categories: vec![],
            shortcut_category_assignments: std::collections::HashMap::new(),
            check_updates_on_startup: true,
        }
    }
}

fn default_true() -> bool {
    true
}

fn default_watcher_interval() -> u64 {
    3
}

// ─── Runtime State ───────────────────────────────────────────────────────────

#[derive(Clone, Debug, Default)]
pub struct MonitorInfo {
    pub rect: RECT,
    pub device_name: String,
    pub hardware_name: Option<String>,
    pub target_id: Option<u32>,
    pub is_active: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[allow(dead_code)]
pub struct SavedMonitorPos {
    pub device_name: String,
    pub rect: SerializableRect,
}
