use eframe::egui;
use crate::ui::icons::regular;

use crate::app::WindowManagerApp;
use crate::ui::helpers;

// ─── Status / Log Bar ────────────────────────────────────────────────────────

pub fn draw_status_bar(app: &mut WindowManagerApp, ui: &mut egui::Ui) {
    let theme = &app.theme;

    // ── Header row with Clear button ─────────────────────────────────────────
    ui.horizontal(|ui| {
        helpers::section_header(ui, theme, regular::LIST_BULLETS, "Activity Log", None);

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

    helpers::nested_card(ui, theme, |ui| {
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
                    } else {
                        theme.text_secondary
                    };
                    ui.label(egui::RichText::new(line).color(color).small());
                }
            });
    });
}
