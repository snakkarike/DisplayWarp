use eframe::egui;
use egui_phosphor::regular;

use crate::app::WindowManagerApp;

// ─── Status / Log Bar ────────────────────────────────────────────────────────

pub fn draw_status_bar(app: &mut WindowManagerApp, ui: &mut egui::Ui) {
    // ── Header row with Clear button ─────────────────────────────────────────
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(format!("{} Activity Log", regular::LIST_BULLETS))
                .size(14.0)
                .strong(),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .add(egui::Button::new(
                    egui::RichText::new(format!("{} Clear", regular::TRASH)).small(),
                ))
                .clicked()
            {
                let mut l = app.status_log.lock();
                l.clear();
                l.push("🗑 Log cleared.".to_string());
            }
        });
    });

    ui.add_space(4.0);

    let log = app.status_log.lock().clone();

    egui::Frame::NONE
        .inner_margin(egui::Margin::same(6))
        .corner_radius(egui::CornerRadius::same(4))
        .fill(if app.dark_mode {
            egui::Color32::from_gray(25)
        } else {
            egui::Color32::from_rgb(241, 245, 249)
        })
        .stroke(egui::Stroke::new(
            1.0,
            if app.dark_mode {
                egui::Color32::from_gray(45)
            } else {
                egui::Color32::from_rgb(226, 232, 240)
            },
        ))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.set_min_height(ui.available_height());

            egui::ScrollArea::vertical()
                .id_salt("status_log_scroll")
                .stick_to_bottom(true)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    for line in &log {
                        // Strip the timestamp prefix for emoji color-matching
                        let content = if line.starts_with('[') {
                            line.splitn(3, "] ").nth(1).unwrap_or(line.as_str())
                        } else {
                            line.as_str()
                        };
                        let color = if content.starts_with('✅') {
                            egui::Color32::LIGHT_GREEN
                        } else if content.starts_with('❌') {
                            egui::Color32::LIGHT_RED
                        } else if content.starts_with('⚠') {
                            egui::Color32::YELLOW
                        } else if content.starts_with('🎵') {
                            egui::Color32::from_rgb(134, 239, 172)
                        } else if content.starts_with('🔍') {
                            egui::Color32::from_rgb(147, 197, 253)
                        } else if content.starts_with('⏳') {
                            egui::Color32::from_rgb(253, 224, 71)
                        } else if content.starts_with('🗑') {
                            egui::Color32::from_rgb(200, 200, 200)
                        } else if content.starts_with('🚀') {
                            egui::Color32::from_rgb(167, 139, 250)
                        } else if app.dark_mode {
                            egui::Color32::GRAY
                        } else {
                            egui::Color32::from_gray(100)
                        };
                        ui.label(egui::RichText::new(line).color(color).small());
                    }
                });
        });
}
