use eframe::egui;

use crate::app::WindowManagerApp;

// ─── Status / Log Bar ────────────────────────────────────────────────────────

pub fn draw_status_bar(app: &WindowManagerApp, ui: &mut egui::Ui) {
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
                        let color = if line.starts_with('✅') {
                            egui::Color32::LIGHT_GREEN
                        } else if line.starts_with('❌') {
                            egui::Color32::LIGHT_RED
                        } else if line.starts_with('⚠') {
                            egui::Color32::YELLOW
                        } else if line.starts_with('🎵') {
                            egui::Color32::from_rgb(134, 239, 172)
                        } else if line.starts_with('🔍') {
                            egui::Color32::from_rgb(147, 197, 253)
                        } else if line.starts_with('⏳') {
                            egui::Color32::from_rgb(253, 224, 71)
                        } else if app.dark_mode {
                            egui::Color32::GRAY
                        } else {
                            egui::Color32::from_gray(100)
                        };
                        ui.label(egui::RichText::new(line).color(color));
                    }
                });
        });
}
