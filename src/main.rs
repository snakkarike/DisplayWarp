#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::ptr;
use std::sync::Arc;
use windows::Win32::Foundation::{BOOL, FALSE, HWND, LPARAM, RECT, TRUE};
use windows::Win32::Graphics::Gdi::{
    CDS_GLOBAL, CDS_NORESET, CDS_SET_PRIMARY, CDS_TYPE, CDS_UPDATEREGISTRY,
    ChangeDisplaySettingsExW, DEVMODEW, DM_POSITION, ENUM_CURRENT_SETTINGS, EnumDisplayMonitors,
    EnumDisplaySettingsW, GetMonitorInfoW, HDC, HMONITOR, MONITOR_DEFAULTTONEAREST, MONITORINFOEXW,
    MonitorFromWindow,
};
use windows::Win32::System::Threading::{
    OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SYNCHRONIZE,
    QueryFullProcessImageNameW, WaitForSingleObject,
};
use windows::Win32::UI::WindowsAndMessaging::{
    BringWindowToTop, EnumWindows, GWL_EXSTYLE, GetWindowLongW, GetWindowRect,
    GetWindowTextLengthW, GetWindowThreadProcessId, HWND_TOP, IsWindowVisible, SW_MAXIMIZE,
    SW_RESTORE, SWP_FRAMECHANGED, SWP_SHOWWINDOW, SetForegroundWindow, SetWindowPos, ShowWindow,
    WS_EX_TOOLWINDOW,
};
use windows::core::PCWSTR;

// ─── Data Models ────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug)]
struct AppProfile {
    name: String,
    exe_path: PathBuf,
    /// Windows device name of the target monitor, e.g. "\\.\DISPLAY2"
    target_monitor_name: String,
    /// Cached rect of the target monitor at save time (used as fallback)
    #[serde(default)]
    target_monitor_rect: Option<SerializableRect>,
    /// Optional: if the launcher spawns a different process (e.g. a game),
    /// set this to that process' exe name, e.g. "Diablo IV.exe".
    /// Leave None to track the launched process itself.
    #[serde(default)]
    window_process_name: Option<String>,
    /// For exclusive fullscreen games: temporarily make the target monitor
    /// primary before launching, then restore after the process exits.
    #[serde(default)]
    force_primary: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
struct SerializableRect {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

#[derive(Serialize, Deserialize, Default)]
struct SavedData {
    profiles: Vec<AppProfile>,
}

// ─── Runtime State ───────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
struct MonitorInfo {
    rect: RECT,
    device_name: String,
}

struct WindowManagerApp {
    monitors: Vec<MonitorInfo>,
    data: SavedData,
    // ── New profile form state ──
    new_profile_exe: Option<PathBuf>,
    selected_mon_idx: usize,
    new_profile_window_process: String,
    new_profile_force_primary: bool,
    // ── Edit profile form state ──
    editing_profile_idx: Option<usize>,
    edit_profile_exe: Option<PathBuf>,
    edit_profile_mon_idx: usize,
    edit_profile_window_process: String,
    edit_profile_force_primary: bool,
    // ── Shared ──
    status_message: Arc<parking_lot::Mutex<String>>,
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
    fn refresh_monitors(&mut self) {
        self.monitors = get_all_monitors();
    }

    fn load_data(&mut self) {
        if let Ok(bytes) = std::fs::read("monitor_config.json") {
            if let Ok(decoded) = serde_json::from_slice::<SavedData>(&bytes) {
                self.data = decoded;
            }
        }
    }

    fn save_data(&self) {
        if let Ok(json) = serde_json::to_string_pretty(&self.data) {
            let _ = std::fs::write("monitor_config.json", json);
        }
    }

    /// Find the live rect for a monitor. Looks up the device name in active monitors.
    fn find_monitor_rect(monitors: &[MonitorInfo], device_name: &str) -> Option<RECT> {
        monitors
            .iter()
            .find(|m| m.device_name == device_name)
            .map(|m| m.rect)
    }

    /// Launch a profile.
    /// - If force_primary: change the primary monitor first, launch, then restore when process exits.
    /// - Otherwise: launch and use the window-moving strategy.
    fn launch_profile(profile: &AppProfile, status: Arc<parking_lot::Mutex<String>>) {
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

    // ─── UI helpers ─────────────────────────────────────────────────────────

    fn draw_layout_preview(&self, ui: &mut egui::Ui) {
        let (rect, _) = ui.allocate_at_least(
            egui::vec2(ui.available_width(), 140.0),
            egui::Sense::hover(),
        );
        let painter = ui.painter_at(rect);

        if self.monitors.is_empty() {
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "No monitors detected",
                egui::FontId::proportional(14.0),
                egui::Color32::GRAY,
            );
            return;
        }

        let min_x = self.monitors.iter().map(|m| m.rect.left).min().unwrap_or(0);
        let max_x = self
            .monitors
            .iter()
            .map(|m| m.rect.right)
            .max()
            .unwrap_or(1);
        let min_y = self.monitors.iter().map(|m| m.rect.top).min().unwrap_or(0);
        let max_y = self
            .monitors
            .iter()
            .map(|m| m.rect.bottom)
            .max()
            .unwrap_or(1);

        let width = (max_x - min_x) as f32;
        let height = (max_y - min_y) as f32;
        let scale = (rect.width() / width).min(rect.height() / height) * 0.85;
        let center = rect.center();

        for (i, m) in self.monitors.iter().enumerate() {
            let is_selected = i == self.selected_mon_idx;
            let m_rect = egui::Rect::from_min_max(
                center
                    + egui::vec2(
                        (m.rect.left - min_x) as f32 * scale - (width * scale / 2.0),
                        (m.rect.top - min_y) as f32 * scale - (height * scale / 2.0),
                    ),
                center
                    + egui::vec2(
                        (m.rect.right - min_x) as f32 * scale - (width * scale / 2.0),
                        (m.rect.bottom - min_y) as f32 * scale - (height * scale / 2.0),
                    ),
            );

            let color = if is_selected {
                egui::Color32::from_rgb(50, 200, 100)
            } else if m.rect.left == 0 && m.rect.top == 0 {
                egui::Color32::from_rgb(0, 130, 220)
            } else {
                egui::Color32::from_gray(70)
            };

            painter.rect_filled(m_rect, 3.0, color);
            painter.rect_stroke(
                m_rect,
                3.0,
                egui::Stroke::new(if is_selected { 2.0 } else { 1.0 }, egui::Color32::WHITE),
            );
            painter.text(
                m_rect.center(),
                egui::Align2::CENTER_CENTER,
                format!("{}", i + 1),
                egui::FontId::proportional(16.0),
                egui::Color32::WHITE,
            );
            // Show resolution below number
            let w = m.rect.right - m.rect.left;
            let h = m.rect.bottom - m.rect.top;
            painter.text(
                m_rect.center() + egui::vec2(0.0, 14.0),
                egui::Align2::CENTER_CENTER,
                format!("{}×{}", w, h),
                egui::FontId::proportional(9.0),
                egui::Color32::from_white_alpha(180),
            );
        }
    }
}

// ─── eframe App ──────────────────────────────────────────────────────────────

impl eframe::App for WindowManagerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Poll for status updates from background threads
        ctx.request_repaint_after(std::time::Duration::from_millis(500));

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Monitor Launcher");
            ui.add_space(4.0);

            // ── Monitor layout preview ──────────────────────────────────
            ui.group(|ui| {
                ui.label("📺 Monitor Layout  (green = selected target):");
                self.draw_layout_preview(ui);
                ui.horizontal(|ui| {
                    if ui.button("� Refresh Monitors").clicked() {
                        self.refresh_monitors();
                    }
                    for (i, m) in self.monitors.iter().enumerate() {
                        let label = format!(
                            "  Mon {} ({}×{})",
                            i + 1,
                            m.rect.right - m.rect.left,
                            m.rect.bottom - m.rect.top
                        );
                        ui.selectable_value(&mut self.selected_mon_idx, i, label);
                    }
                });
            });

            ui.add_space(6.0);

            // ── Profiles ────────────────────────────────────────────────
            ui.label("🗂 Saved Profiles:");
            egui::ScrollArea::vertical()
                .max_height(220.0)
                .show(ui, |ui| {
                    let mut to_remove: Option<usize> = None;
                    let profiles = self.data.profiles.clone();
                    for (i, p) in profiles.iter().enumerate() {
                        let is_editing = self.editing_profile_idx == Some(i);

                        if is_editing {
                            // ── Inline edit form ────────────────────────
                            ui.group(|ui| {
                                ui.label(
                                    egui::RichText::new(format!("✏ Editing: {}", p.name))
                                        .color(egui::Color32::YELLOW),
                                );

                                ui.horizontal(|ui| {
                                    if ui.button("📁 Change EXE").clicked() {
                                        self.edit_profile_exe = rfd::FileDialog::new()
                                            .add_filter("Executable", &["exe"])
                                            .pick_file();
                                    }
                                    let shown = self
                                        .edit_profile_exe
                                        .as_ref()
                                        .map(|e| {
                                            e.file_name().unwrap().to_string_lossy().into_owned()
                                        })
                                        .unwrap_or_else(|| {
                                            p.exe_path
                                                .file_name()
                                                .unwrap()
                                                .to_string_lossy()
                                                .into_owned()
                                        });
                                    ui.label(
                                        egui::RichText::new(shown)
                                            .color(egui::Color32::LIGHT_GREEN),
                                    );
                                });

                                ui.horizontal(|ui| {
                                    ui.label("Target monitor:");
                                    egui::ComboBox::from_id_salt(format!("edit_mon_{i}"))
                                        .selected_text(if self.monitors.is_empty() {
                                            "No monitors".to_string()
                                        } else {
                                            format!("Monitor {}", self.edit_profile_mon_idx + 1)
                                        })
                                        .show_ui(ui, |ui| {
                                            for (mi, m) in self.monitors.iter().enumerate() {
                                                let w = m.rect.right - m.rect.left;
                                                let h = m.rect.bottom - m.rect.top;
                                                ui.selectable_value(
                                                    &mut self.edit_profile_mon_idx,
                                                    mi,
                                                    format!("Monitor {} ({}×{})", mi + 1, w, h),
                                                );
                                            }
                                        });
                                });

                                ui.label("Window process (optional):");
                                ui.add(
                                    egui::TextEdit::singleline(
                                        &mut self.edit_profile_window_process,
                                    )
                                    .hint_text("e.g. Diablo IV.exe — leave blank if not needed")
                                    .desired_width(f32::INFINITY),
                                );

                                ui.horizontal(|ui| {
                                    ui.checkbox(
                                        &mut self.edit_profile_force_primary,
                                        "🔒 Force Primary Monitor",
                                    );
                                    ui.label(
                                        egui::RichText::new("(for exclusive fullscreen games)")
                                            .small()
                                            .color(egui::Color32::GRAY),
                                    );
                                });

                                ui.horizontal(|ui| {
                                    if ui.button("✅ Save Changes").clicked() {
                                        if let Some(idx) = self.editing_profile_idx {
                                            let prof = &mut self.data.profiles[idx];
                                            if let Some(new_exe) = self.edit_profile_exe.take() {
                                                prof.name = new_exe
                                                    .file_name()
                                                    .unwrap()
                                                    .to_string_lossy()
                                                    .into_owned();
                                                prof.exe_path = new_exe;
                                            }
                                            let mon = &self.monitors[self.edit_profile_mon_idx];
                                            prof.target_monitor_name = mon.device_name.clone();
                                            prof.target_monitor_rect = Some(SerializableRect {
                                                left: mon.rect.left,
                                                top: mon.rect.top,
                                                right: mon.rect.right,
                                                bottom: mon.rect.bottom,
                                            });
                                            let proc =
                                                self.edit_profile_window_process.trim().to_string();
                                            prof.window_process_name =
                                                if proc.is_empty() { None } else { Some(proc) };
                                            prof.force_primary = self.edit_profile_force_primary;
                                            self.save_data();
                                        }
                                        self.editing_profile_idx = None;
                                        self.edit_profile_exe = None;
                                        self.edit_profile_window_process.clear();
                                        self.edit_profile_force_primary = false;
                                    }
                                    if ui.button("✖ Cancel").clicked() {
                                        self.editing_profile_idx = None;
                                        self.edit_profile_exe = None;
                                        self.edit_profile_window_process.clear();
                                        self.edit_profile_force_primary = false;
                                    }
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            if ui.button("🗑 Delete").clicked() {
                                                to_remove = Some(i);
                                                self.editing_profile_idx = None;
                                            }
                                        },
                                    );
                                });
                            });
                        } else {
                            // ── Normal profile card ──────────────────────
                            ui.group(|ui| {
                                ui.horizontal(|ui| {
                                    ui.label(format!("🚀 {}", p.name));
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            if ui.button("🗑 Delete").clicked() {
                                                to_remove = Some(i);
                                            }
                                            if ui.button("✏ Edit").clicked() {
                                                // Populate edit state from the profile
                                                self.editing_profile_idx = Some(i);
                                                self.edit_profile_exe = None;
                                                self.edit_profile_mon_idx = self
                                                    .monitors
                                                    .iter()
                                                    .position(|m| {
                                                        m.device_name == p.target_monitor_name
                                                    })
                                                    .unwrap_or(0);
                                                self.edit_profile_window_process = p
                                                    .window_process_name
                                                    .clone()
                                                    .unwrap_or_default();
                                                self.edit_profile_force_primary = p.force_primary;
                                            }
                                            if ui.button("▶ Launch").clicked() {
                                                Self::launch_profile(
                                                    p,
                                                    Arc::clone(&self.status_message),
                                                );
                                            }
                                        },
                                    );
                                });
                                let proc_label = p
                                    .window_process_name
                                    .as_deref()
                                    .map(|s| format!(" | watch: {s}"))
                                    .unwrap_or_default();
                                let primary_label = if p.force_primary {
                                    " | 🔒 force primary"
                                } else {
                                    ""
                                };
                                ui.label(
                                    egui::RichText::new(format!(
                                        "  {} → {}{}{}",
                                        p.exe_path.display(),
                                        p.target_monitor_name,
                                        proc_label,
                                        primary_label,
                                    ))
                                    .small()
                                    .color(egui::Color32::GRAY),
                                );
                            });
                        }
                    }
                    if let Some(i) = to_remove {
                        self.data.profiles.remove(i);
                        self.save_data();
                    }
                });

            ui.add_space(6.0);

            // ── Create new profile ───────────────────────────────────────
            ui.group(|ui| {
                ui.label("➕ New Profile:");
                ui.horizontal(|ui| {
                    if ui.button("� Select EXE").clicked() {
                        self.new_profile_exe = rfd::FileDialog::new()
                            .add_filter("Executable", &["exe"])
                            .pick_file();
                    }
                    if let Some(p) = &self.new_profile_exe {
                        ui.label(
                            egui::RichText::new(p.file_name().unwrap().to_string_lossy())
                                .color(egui::Color32::LIGHT_GREEN),
                        );
                    } else {
                        ui.label(
                            egui::RichText::new("No file selected").color(egui::Color32::GRAY),
                        );
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("Target monitor:");
                    egui::ComboBox::from_id_salt("target_mon")
                        .selected_text(if self.monitors.is_empty() {
                            "No monitors".to_string()
                        } else {
                            format!("Monitor {}", self.selected_mon_idx + 1)
                        })
                        .show_ui(ui, |ui| {
                            for (i, m) in self.monitors.iter().enumerate() {
                                let w = m.rect.right - m.rect.left;
                                let h = m.rect.bottom - m.rect.top;
                                ui.selectable_value(
                                    &mut self.selected_mon_idx,
                                    i,
                                    format!("Monitor {} ({}×{})", i + 1, w, h),
                                );
                            }
                        });
                });

                ui.horizontal(|ui| {
                    ui.label("Window process");
                    ui.label(
                        egui::RichText::new("(optional — for launchers that open a different exe)")
                            .small()
                            .color(egui::Color32::GRAY),
                    );
                });
                ui.horizontal(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut self.new_profile_window_process)
                            .hint_text("e.g. Diablo IV.exe  — leave blank if not needed")
                            .desired_width(f32::INFINITY),
                    );
                });

                ui.horizontal(|ui| {
                    ui.checkbox(
                        &mut self.new_profile_force_primary,
                        "🔒 Force Primary Monitor",
                    );
                    ui.label(
                        egui::RichText::new("(for exclusive fullscreen games like Unravel Two)")
                            .small()
                            .color(egui::Color32::GRAY),
                    );
                });

                if ui
                    .add_enabled(
                        self.new_profile_exe.is_some() && !self.monitors.is_empty(),
                        egui::Button::new("💾 Save Profile"),
                    )
                    .clicked()
                {
                    let exe = self.new_profile_exe.clone().unwrap();
                    let mon = &self.monitors[self.selected_mon_idx];
                    let proc_name = self.new_profile_window_process.trim().to_string();
                    self.data.profiles.push(AppProfile {
                        name: exe.file_name().unwrap().to_string_lossy().into_owned(),
                        exe_path: exe,
                        target_monitor_name: mon.device_name.clone(),
                        target_monitor_rect: Some(SerializableRect {
                            left: mon.rect.left,
                            top: mon.rect.top,
                            right: mon.rect.right,
                            bottom: mon.rect.bottom,
                        }),
                        window_process_name: if proc_name.is_empty() {
                            None
                        } else {
                            Some(proc_name)
                        },
                        force_primary: self.new_profile_force_primary,
                    });
                    self.new_profile_exe = None;
                    self.new_profile_window_process.clear();
                    self.new_profile_force_primary = false;
                    self.save_data();
                }
            });

            ui.add_space(6.0);

            // ── Status bar ───────────────────────────────────────────────
            let status = self.status_message.lock().clone();
            ui.separator();
            ui.label(
                egui::RichText::new(&status)
                    .small()
                    .color(if status.starts_with('✅') {
                        egui::Color32::LIGHT_GREEN
                    } else if status.starts_with('❌') {
                        egui::Color32::LIGHT_RED
                    } else if status.starts_with('⚠') {
                        egui::Color32::YELLOW
                    } else {
                        egui::Color32::GRAY
                    }),
            );
        });
    }
}

// ─── Win32 Helpers ───────────────────────────────────────────────────────────

/// Poll for a visible, top-level window owned by `pid`. Waits up to `timeout_ms`.
fn wait_for_window(pid: u32, timeout_ms: u64) -> Option<HWND> {
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(timeout_ms);
    loop {
        if let Some(h) = find_main_window_by_pid(pid) {
            return Some(h);
        }
        if std::time::Instant::now() >= deadline {
            return None;
        }
        std::thread::sleep(std::time::Duration::from_millis(300));
    }
}

/// Poll for a visible window whose owning process exe name matches `process_name` (case-insensitive).
/// Useful when a launcher spawns a different child process (e.g. Battle.net → actual game).
fn wait_for_window_by_name(process_name: &str, timeout_ms: u64) -> Option<HWND> {
    let target = process_name.to_lowercase();
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(timeout_ms);
    loop {
        if let Some(h) = find_window_by_process_name(&target) {
            return Some(h);
        }
        if std::time::Instant::now() >= deadline {
            return None;
        }
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
}

struct WindowCandidate {
    hwnd: HWND,
    score: i32,
}

struct FindWindowByNameData {
    target_name: String, // lowercase
    candidates: Vec<WindowCandidate>,
}

fn find_window_by_process_name(target_lowercase: &str) -> Option<HWND> {
    let mut data = FindWindowByNameData {
        target_name: target_lowercase.to_string(),
        candidates: Vec::new(),
    };
    unsafe {
        let _ = EnumWindows(
            Some(enum_window_by_name_callback),
            LPARAM(&mut data as *mut _ as isize),
        );
    }
    // Pick highest-scoring candidate
    data.candidates
        .into_iter()
        .max_by_key(|c| c.score)
        .map(|c| c.hwnd)
}

unsafe extern "system" fn enum_window_by_name_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
    unsafe {
        if !IsWindowVisible(hwnd).as_bool() {
            return TRUE;
        }
        let data = &mut *(lparam.0 as *mut FindWindowByNameData);
        let mut pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if pid == 0 {
            return TRUE;
        }
        if let Ok(hproc) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) {
            let mut buf = [0u16; 512];
            let mut len = buf.len() as u32;
            if QueryFullProcessImageNameW(
                hproc,
                PROCESS_NAME_WIN32,
                windows::core::PWSTR(buf.as_mut_ptr()),
                &mut len,
            )
            .is_ok()
            {
                let full_path = String::from_utf16_lossy(&buf[..len as usize]).to_lowercase();
                let exe_name = full_path.split(['/', '\\']).last().unwrap_or("");
                if exe_name == data.target_name {
                    // Score this candidate:
                    let mut score: i32 = 0;
                    // Prefer windows with a title (loader windows often have no title)
                    let title_len = GetWindowTextLengthW(hwnd);
                    if title_len > 0 {
                        score += 20;
                    }
                    // Penalise tool windows (overlays, toasts, etc)
                    let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;
                    if ex_style & WS_EX_TOOLWINDOW.0 != 0 {
                        score -= 50;
                    }
                    // Prefer larger windows (game window will be biggest)
                    let mut wr = RECT::default();
                    if GetWindowRect(hwnd, &mut wr).is_ok() {
                        let area = ((wr.right - wr.left) * (wr.bottom - wr.top)).max(0);
                        score += (area / 10_000).min(100); // cap at +100
                    }
                    data.candidates.push(WindowCandidate { hwnd, score });
                }
            }
            let _ = windows::Win32::Foundation::CloseHandle(hproc);
        }
        TRUE // always keep enumerating to collect all candidates
    }
}

struct FindWindowData {
    pid: u32,
    hwnd: HWND,
}

fn find_main_window_by_pid(pid: u32) -> Option<HWND> {
    let mut data = FindWindowData {
        pid,
        hwnd: HWND(ptr::null_mut()),
    };
    unsafe {
        let _ = EnumWindows(
            Some(enum_window_callback),
            LPARAM(&mut data as *mut _ as isize),
        );
    }
    if data.hwnd.0.is_null() {
        None
    } else {
        Some(data.hwnd)
    }
}

unsafe extern "system" fn enum_window_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
    unsafe {
        let data = &mut *(lparam.0 as *mut FindWindowData);
        let mut window_pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut window_pid));
        if window_pid == data.pid && IsWindowVisible(hwnd).as_bool() {
            data.hwnd = hwnd;
            FALSE // Stop enumeration — we have our window
        } else {
            TRUE
        }
    }
}

/// Resolve the HMONITOR for the centre of a rect.
fn monitor_for_rect(rect: RECT) -> HMONITOR {
    use windows::Win32::Foundation::POINT;
    use windows::Win32::Graphics::Gdi::{MONITOR_DEFAULTTONEAREST as MDN, MonitorFromPoint};
    let w = rect.right - rect.left;
    let h = rect.bottom - rect.top;
    unsafe {
        MonitorFromPoint(
            POINT {
                x: rect.left + w / 2,
                y: rect.top + h / 2,
            },
            MDN,
        )
    }
}

/// Move a window to the target monitor rect, retrying until it sticks,
/// then watch it for 45 seconds and enforce position if the game moves it back.
fn move_to_monitor(hwnd: HWND, target_rect: RECT) {
    let w = target_rect.right - target_rect.left;
    let h = target_rect.bottom - target_rect.top;
    let target_mon = monitor_for_rect(target_rect);

    // ── Phase 1: aggressive initial placement (first ~6 s) ────────────────
    for attempt in 0..12u32 {
        unsafe {
            if attempt > 0 {
                std::thread::sleep(std::time::Duration::from_millis(500));
            }
            let _ = BringWindowToTop(hwnd);
            let _ = SetForegroundWindow(hwnd);
            let _ = ShowWindow(hwnd, SW_RESTORE);
            std::thread::sleep(std::time::Duration::from_millis(60));
            let _ = SetWindowPos(
                hwnd,
                HWND_TOP,
                target_rect.left,
                target_rect.top,
                w,
                h,
                SWP_SHOWWINDOW | SWP_FRAMECHANGED,
            );
            let _ = ShowWindow(hwnd, SW_MAXIMIZE);
            std::thread::sleep(std::time::Duration::from_millis(100));
            if MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST) == target_mon {
                break;
            }
        }
    }

    // ── Phase 2: gentle keep-alive watcher for 45 s ─────────────────────
    // Only re-positions — does NOT call SW_MAXIMIZE — to avoid minimize/maximize flicker
    // on exclusive-fullscreen games that still ignore SetWindowPos.
    let watch_deadline = std::time::Instant::now() + std::time::Duration::from_secs(45);
    while std::time::Instant::now() < watch_deadline {
        std::thread::sleep(std::time::Duration::from_millis(1000));
        unsafe {
            if MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST) != target_mon {
                let _ = BringWindowToTop(hwnd);
                let _ = SetForegroundWindow(hwnd);
                let _ = ShowWindow(hwnd, SW_RESTORE);
                std::thread::sleep(std::time::Duration::from_millis(60));
                let _ = SetWindowPos(
                    hwnd,
                    HWND_TOP,
                    target_rect.left,
                    target_rect.top,
                    w,
                    h,
                    SWP_SHOWWINDOW | SWP_FRAMECHANGED,
                );
                // No SW_MAXIMIZE here — avoids flicker on exclusive-fullscreen games
            }
        }
    }
}

// ─── Primary-Monitor Switching ──────────────────────────────────────────────────

#[derive(Clone)]
struct SavedMonitorPos {
    device_name: String,
    rect: RECT,
}

/// Make `target_device_name` the primary monitor by shifting all monitor
/// coordinates so the target sits at (0, 0).
fn switch_primary_to(target_device_name: &str, monitors: &[MonitorInfo]) -> bool {
    let target = match monitors
        .iter()
        .find(|m| m.device_name == target_device_name)
    {
        Some(m) => m,
        None => return false,
    };
    let offset_x = target.rect.left;
    let offset_y = target.rect.top;

    unsafe {
        for mon in monitors {
            let name_u16: Vec<u16> = mon.device_name.encode_utf16().chain(Some(0)).collect();
            let mut dev_mode = std::mem::zeroed::<DEVMODEW>();
            dev_mode.dmSize = std::mem::size_of::<DEVMODEW>() as u16;
            if !EnumDisplaySettingsW(
                PCWSTR(name_u16.as_ptr()),
                ENUM_CURRENT_SETTINGS,
                &mut dev_mode,
            )
            .as_bool()
            {
                continue;
            }
            dev_mode.dmFields = DM_POSITION;
            dev_mode.Anonymous1.Anonymous2.dmPosition.x = mon.rect.left - offset_x;
            dev_mode.Anonymous1.Anonymous2.dmPosition.y = mon.rect.top - offset_y;

            let mut flags = CDS_UPDATEREGISTRY | CDS_NORESET | CDS_GLOBAL;
            if mon.device_name == target_device_name {
                flags |= CDS_SET_PRIMARY;
            }
            ChangeDisplaySettingsExW(
                PCWSTR(name_u16.as_ptr()),
                Some(&dev_mode),
                HWND(ptr::null_mut()),
                flags,
                None,
            );
        }
        // Flush all changes at once
        let _ = ChangeDisplaySettingsExW(
            PCWSTR(ptr::null()),
            None,
            HWND(ptr::null_mut()),
            CDS_TYPE(0),
            None,
        );
    }
    true
}

/// Restore monitor positions from a saved snapshot.
fn restore_monitor_layout(snapshot: &[SavedMonitorPos]) {
    unsafe {
        for saved in snapshot {
            let name_u16: Vec<u16> = saved.device_name.encode_utf16().chain(Some(0)).collect();
            let mut dev_mode = std::mem::zeroed::<DEVMODEW>();
            dev_mode.dmSize = std::mem::size_of::<DEVMODEW>() as u16;
            if !EnumDisplaySettingsW(
                PCWSTR(name_u16.as_ptr()),
                ENUM_CURRENT_SETTINGS,
                &mut dev_mode,
            )
            .as_bool()
            {
                continue;
            }
            dev_mode.dmFields = DM_POSITION;
            dev_mode.Anonymous1.Anonymous2.dmPosition.x = saved.rect.left;
            dev_mode.Anonymous1.Anonymous2.dmPosition.y = saved.rect.top;
            // Mark as primary if it was primary (i.e., its top-left was (0,0))
            let mut flags = CDS_UPDATEREGISTRY | CDS_NORESET | CDS_GLOBAL;
            if saved.rect.left == 0 && saved.rect.top == 0 {
                flags |= CDS_SET_PRIMARY;
            }
            ChangeDisplaySettingsExW(
                PCWSTR(name_u16.as_ptr()),
                Some(&dev_mode),
                HWND(ptr::null_mut()),
                flags,
                None,
            );
        }
        let _ = ChangeDisplaySettingsExW(
            PCWSTR(ptr::null()),
            None,
            HWND(ptr::null_mut()),
            CDS_TYPE(0),
            None,
        );
    }
}

/// Block until a process (by PID) exits, or forever if it cannot be opened.
fn wait_for_pid_exit(pid: u32) {
    unsafe {
        if let Ok(hproc) = OpenProcess(PROCESS_SYNCHRONIZE, false, pid) {
            let _ = WaitForSingleObject(hproc, u32::MAX);
            let _ = windows::Win32::Foundation::CloseHandle(hproc);
        }
    }
}

/// Enumerate all connected monitors and return their info.
fn get_all_monitors() -> Vec<MonitorInfo> {
    let mut monitors = Vec::new();
    unsafe {
        let _ = EnumDisplayMonitors(
            HDC(ptr::null_mut()),
            None,
            Some(enum_monitor_callback),
            LPARAM(&mut monitors as *mut _ as isize),
        );
    }
    monitors
}

unsafe extern "system" fn enum_monitor_callback(
    hmon: HMONITOR,
    _: HDC,
    _: *mut RECT,
    lparam: LPARAM,
) -> BOOL {
    unsafe {
        let monitors = &mut *(lparam.0 as *mut Vec<MonitorInfo>);
        let mut info = MONITORINFOEXW::default();
        info.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;
        if GetMonitorInfoW(hmon, &mut info.monitorInfo).as_bool() {
            let device_name = String::from_utf16_lossy(&info.szDevice)
                .trim_matches(char::from(0))
                .to_string();
            monitors.push(MonitorInfo {
                rect: info.monitorInfo.rcMonitor,
                device_name,
            });
        }
        TRUE
    }
}

// ─── Entry Point ─────────────────────────────────────────────────────────────

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([480.0, 620.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Monitor Launcher",
        options,
        Box::new(|_| Ok(Box::new(WindowManagerApp::default()))),
    )
}
