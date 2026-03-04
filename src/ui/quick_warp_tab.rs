use std::sync::Arc;

use eframe::egui;
use egui_phosphor::regular;

use crate::app::WindowManagerApp;
use crate::models::{AppProfile, SerializableRect};
use crate::ui::helpers::truncate_text;
use crate::ui::monitor_preview::draw_monitor_preview;

// ─── Move Live Window ────────────────────────────────────────────────────────

pub fn draw_live_process_mover(app: &mut WindowManagerApp, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(format!("{} Move Live Window", regular::MONITOR))
                .size(14.0)
                .strong(),
        );
    });
    ui.label(
        egui::RichText::new("Easily move an open application to a specific monitor.")
            .small()
            .color(if app.dark_mode {
                egui::Color32::from_gray(140)
            } else {
                egui::Color32::from_gray(100)
            }),
    );

    ui.add_space(4.0);

    egui::Frame::NONE
        .inner_margin(egui::Margin::same(8))
        .corner_radius(egui::CornerRadius::same(6))
        .fill(if app.dark_mode {
            egui::Color32::from_rgb(34, 34, 34)
        } else {
            egui::Color32::from_rgb(241, 245, 249)
        })
        .stroke(egui::Stroke::new(
            1.0,
            if app.dark_mode {
                egui::Color32::from_rgb(44, 44, 44)
            } else {
                egui::Color32::from_rgb(226, 232, 240)
            },
        ))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(egui::RichText::new(format!("{} Window Process", regular::FILE)).strong());
            ui.add_space(4.0);
            let current_label = app
                .live_processes
                .get(app.selected_live_process_idx)
                .map(|e| e.label.as_str())
                .unwrap_or("Select Live Process");
            let display_label = truncate_text(current_label, 30);
            ui.add_enabled_ui(!app.live_processes.is_empty(), |ui| {
                egui::ComboBox::from_id_salt("live_proc")
                    .selected_text(display_label)
                    .width(ui.available_width())
                    .height(300.0)
                    .show_ui(ui, |ui| {
                        for (i, entry) in app.live_processes.iter().enumerate() {
                            let item_text = format!(
                                "{} {}",
                                regular::APP_WINDOW,
                                truncate_text(&entry.label, 40)
                            );
                            ui.selectable_value(&mut app.selected_live_process_idx, i, item_text);
                        }
                    });
            });
        });

    ui.add_space(4.0);

    // Target monitor selector
    egui::Frame::NONE
        .inner_margin(egui::Margin::same(8))
        .corner_radius(egui::CornerRadius::same(6))
        .fill(if app.dark_mode {
            egui::Color32::from_rgb(34, 34, 34)
        } else {
            egui::Color32::from_rgb(241, 245, 249)
        })
        .stroke(egui::Stroke::new(
            1.0,
            if app.dark_mode {
                egui::Color32::from_rgb(44, 44, 44)
            } else {
                egui::Color32::from_rgb(226, 232, 240)
            },
        ))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(
                egui::RichText::new(format!("{} Select Target Monitor", regular::MONITOR)).strong(),
            );
            ui.add_space(4.0);
            egui::ComboBox::from_id_salt("live_mon")
                .selected_text(if app.monitors.is_empty() {
                    "No monitors".to_string()
                } else {
                    format!("Monitor {}", app.live_move_mon_idx + 1)
                })
                .width(ui.available_width())
                .show_ui(ui, |ui| {
                    for (i, m) in app.monitors.iter().enumerate() {
                        let w = m.rect.right - m.rect.left;
                        let h = m.rect.bottom - m.rect.top;
                        let hardware_name = app
                            .display_targets
                            .iter()
                            .find(|t| t.device_name == m.device_name)
                            .and_then(|t| t.hardware_name.clone())
                            .unwrap_or_else(|| {
                                m.device_name
                                    .replace("\\\\.\\", "")
                                    .replace("DISPLAY", "Display ")
                            });
                        ui.selectable_value(
                            &mut app.live_move_mon_idx,
                            i,
                            format!("Monitor {} ({}) ({}×{})", i + 1, hardware_name, w, h),
                        );
                    }
                });
        });

    ui.add_space(6.0);

    // Move and Create Profile buttons
    let can_move = !app.live_processes.is_empty() && !app.monitors.is_empty();

    ui.horizontal(|ui| {
        let btn_width = (ui.available_width() - 8.0) / 2.0;
        if ui
            .add_sized(
                [btn_width, 30.0],
                egui::Button::new(
                    egui::RichText::new(format!("{} Move Process", regular::ARROWS_OUT_SIMPLE))
                        .strong(),
                ),
            )
            .clicked()
            && can_move
        {
            if let Some(entry) = app.live_processes.get(app.selected_live_process_idx) {
                let hwnd = entry.hwnd;
                let target = app.monitors[app.live_move_mon_idx].rect;
                WindowManagerApp::move_live_window(
                    hwnd,
                    target.into(),
                    Arc::clone(&app.status_message),
                    Arc::clone(&app.status_log),
                );
            }
        }

        if ui
            .add_sized(
                [btn_width, 30.0],
                egui::Button::new(
                    egui::RichText::new(format!("{} Create Profile", regular::PLUS)).strong(),
                ),
            )
            .clicked()
            && can_move
        {
            if let Some(entry) = app.live_processes.get(app.selected_live_process_idx) {
                if let Some(path) = &entry.exe_path {
                    let mon = if let Some(m) = app.monitors.get(app.live_move_mon_idx) {
                        m
                    } else if let Some(m) = app.monitors.first() {
                        m
                    } else {
                        return;
                    };
                    let exe = path.clone();
                    app.data.lock().profiles.push(AppProfile {
                        name: exe.file_name().unwrap().to_string_lossy().into_owned(),
                        exe_path: exe,
                        target_monitor_name: mon.device_name.clone(),
                        target_monitor_rect: Some(SerializableRect {
                            left: mon.rect.left,
                            top: mon.rect.top,
                            right: mon.rect.right,
                            bottom: mon.rect.bottom,
                        }),
                        window_process_name: None,
                        launch_args: None,
                        window_title_match: None,
                        force_primary: false,
                        persistent_monitor: false,
                        target_audio_device_id: None,
                    });
                    app.save_data();
                    *app.status_message.lock() = "✅ Profile created from live process.".into();
                } else {
                    *app.status_message.lock() =
                        "❌ Could not determine executable path for this process.".into();
                }
            }
        }
    });
    ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
        let width = ui.available_width();

        if ui
            .add_sized(
                [width, 30.0],
                egui::Button::new(format!("{} Refresh", regular::ARROW_CLOCKWISE)),
            )
            .clicked()
        {
            app.refresh_live_processes();
        }
    });
}

// ─── Main Warp Tab ──────────────────────────────────────────────────────────

pub fn draw_quick_warp_tab(app: &mut WindowManagerApp, ui: &mut egui::Ui) {
    let preview_idx = if app.editing_profile_idx.is_some() {
        app.edit_profile_mon_idx
    } else {
        app.selected_mon_idx
    };
    draw_monitor_preview(app, ui, Some(preview_idx));

    ui.add_space(8.0);

    ui.columns(1, |cols| {
        let _col_height = cols[0].available_height();

        cols[0].vertical(|ui| {
            let h = ui.available_height();
            let w = ui.available_width();
            ui.set_min_size(egui::vec2(w, h));
            egui::Frame::group(ui.style())
                .inner_margin(egui::Margin::same(12))
                .corner_radius(egui::CornerRadius::same(8))
                .fill(if app.dark_mode {
                    egui::Color32::from_rgb(25, 25, 25)
                } else {
                    egui::Color32::from_rgb(248, 250, 252)
                })
                .stroke(egui::Stroke::new(
                    1.0,
                    if app.dark_mode {
                        egui::Color32::from_rgb(44, 44, 44)
                    } else {
                        egui::Color32::from_rgb(226, 232, 240)
                    },
                ))
                .show(ui, |ui| {
                    ui.set_min_size(egui::vec2(ui.available_width(), h));
                    draw_live_process_mover(app, ui);
                });
        });
    });
}
