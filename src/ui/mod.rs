pub mod panels;

use eframe::egui;

use crate::app::WindowManagerApp;

// ─── eframe App impl ─────────────────────────────────────────────────────────

impl eframe::App for WindowManagerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Poll for status updates from background threads
        ctx.request_repaint_after(std::time::Duration::from_millis(500));

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("DisplayWarp");
            ui.add_space(4.0);

            // ── Monitor layout preview ──────────────────────────────────
            // Highlight the monitor that's active in the current context:
            //   - while editing a profile → edit_profile_mon_idx
            //   - otherwise              → selected_mon_idx (new-profile selector)
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

// ─── Layout Preview ──────────────────────────────────────────────────────────

impl WindowManagerApp {
    /// `selected_idx` is which monitor slot to highlight green.
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
