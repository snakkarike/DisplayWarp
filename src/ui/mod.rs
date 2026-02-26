pub mod panels;

use std::sync::atomic::Ordering;

use eframe::egui;

use crate::app::WindowManagerApp;
use crate::tray;

// ─── eframe App impl ─────────────────────────────────────────────────────────

impl eframe::App for WindowManagerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Keep the event loop alive even when hidden (for tray menu polling).
        ctx.request_repaint_after(std::time::Duration::from_millis(500));

        // ── Handle tray menu events ──────────────────────────────────────
        if let Some(id) = tray::poll_menu_event() {
            if let Some(ref tray_items) = self.tray {
                if id == tray_items.show_id {
                    show_native_window(ctx);
                } else if id == tray_items.quit_id {
                    self.watcher_running.store(false, Ordering::Relaxed);
                    std::process::exit(0);
                }
            }
        }

        // ── Intercept close → hide to tray ──────────────────────────────
        // Read the flag FIRST (releases the input lock), then act on it.
        let close_requested = ctx.input(|i| i.viewport().close_requested());
        if close_requested {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            hide_native_window(ctx);
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("DisplayWarp");
            ui.add_space(4.0);

            // ── Monitor layout preview ──────────────────────────────────
            let preview_idx = if self.editing_profile_idx.is_some() {
                self.edit_profile_mon_idx
            } else {
                self.selected_mon_idx
            };

            ui.group(|ui| {
                ui.label("📺 Monitor Layout  (green = selected target):");
                self.draw_layout_preview(ui, preview_idx);
                ui.horizontal(|ui| {
                    if ui.button("🔄 Refresh Monitors").clicked() {
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
                    panels::draw_profiles_list(self, ui);
                });

            ui.add_space(6.0);

            // ── Create new profile ───────────────────────────────────────
            panels::draw_new_profile_form(self, ui);

            ui.add_space(6.0);

            // ── Live-process mover ───────────────────────────────────────
            panels::draw_live_process_mover(self, ui);

            ui.add_space(6.0);

            // ── Status bar ───────────────────────────────────────────────
            panels::draw_status_bar(self, ui);
        });
    }
}

// ─── Native window show/hide ─────────────────────────────────────────────────

/// Hide the eframe window using the native Win32 API.
/// We avoid `Visible(false)` because eframe stops processing events for
/// hidden windows, making it impossible to restore them via tray menu.
fn hide_native_window(ctx: &egui::Context) {
    if let Some(hwnd) = get_eframe_hwnd(ctx) {
        unsafe {
            use windows::Win32::UI::WindowsAndMessaging::{SW_HIDE, ShowWindow};
            let _ = ShowWindow(hwnd, SW_HIDE);
        }
    }
}

/// Show the eframe window using the native Win32 API.
fn show_native_window(ctx: &egui::Context) {
    if let Some(hwnd) = get_eframe_hwnd(ctx) {
        unsafe {
            use windows::Win32::UI::WindowsAndMessaging::{
                BringWindowToTop, SW_SHOW, SetForegroundWindow, ShowWindow,
            };
            // SW_SHOW is the counterpart to SW_HIDE. SW_RESTORE only works on
            // minimized/maximized windows, not hidden ones.
            let _ = ShowWindow(hwnd, SW_SHOW);
            let _ = BringWindowToTop(hwnd);
            let _ = SetForegroundWindow(hwnd);
        }
    }
}

/// Retrieve the native HWND from the egui context via raw-window-handle.
fn get_eframe_hwnd(_ctx: &egui::Context) -> Option<windows::Win32::Foundation::HWND> {
    unsafe {
        use windows::Win32::UI::WindowsAndMessaging::FindWindowW;
        use windows::core::w;
        match FindWindowW(None, w!("Display Warp")) {
            Ok(hwnd) if !hwnd.0.is_null() => Some(hwnd),
            _ => None,
        }
    }
}

// ─── Layout Preview ──────────────────────────────────────────────────────────

impl WindowManagerApp {
    pub fn draw_layout_preview(&self, ui: &mut egui::Ui, selected_idx: usize) {
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
            let is_selected = i == selected_idx;
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
