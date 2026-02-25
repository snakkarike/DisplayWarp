use std::sync::Arc;

use eframe::egui;

use crate::app::WindowManagerApp;
use crate::models::{AppProfile, SerializableRect};

/// Draw individual profile cards (normal view + inline edit form) and the new-profile form.
/// Returns a profile index to remove (if delete was clicked), or None.
pub fn draw_profiles_list(app: &mut WindowManagerApp, ui: &mut egui::Ui) {
    let mut to_remove: Option<usize> = None;
    let profiles = app.data.profiles.clone();
    for (i, p) in profiles.iter().enumerate() {
        let is_editing = app.editing_profile_idx == Some(i);

        if is_editing {
            draw_edit_profile_form(app, ui, i, p, &mut to_remove);
        } else {
            draw_profile_card(app, ui, i, p, &mut to_remove);
        }
    }
    if let Some(i) = to_remove {
        app.data.profiles.remove(i);
        app.save_data();
    }
}

fn draw_edit_profile_form(
    app: &mut WindowManagerApp,
    ui: &mut egui::Ui,
    i: usize,
    p: &AppProfile,
    to_remove: &mut Option<usize>,
) {
    ui.group(|ui| {
        ui.label(
            egui::RichText::new(format!("✏ Editing: {}", p.name)).color(egui::Color32::YELLOW),
        );

        ui.horizontal(|ui| {
            if ui.button("📁 Change EXE").clicked() {
                app.edit_profile_exe = rfd::FileDialog::new()
                    .add_filter("Executable", &["exe"])
                    .pick_file();
            }
            let shown = app
                .edit_profile_exe
                .as_ref()
                .map(|e| e.file_name().unwrap().to_string_lossy().into_owned())
                .unwrap_or_else(|| {
                    p.exe_path
                        .file_name()
                        .unwrap()
                        .to_string_lossy()
                        .into_owned()
                });
            ui.label(egui::RichText::new(shown).color(egui::Color32::LIGHT_GREEN));
        });

        ui.horizontal(|ui| {
            ui.label("Target monitor:");
            egui::ComboBox::from_id_salt(format!("edit_mon_{i}"))
                .selected_text(if app.monitors.is_empty() {
                    "No monitors".to_string()
                } else {
                    format!("Monitor {}", app.edit_profile_mon_idx + 1)
                })
                .show_ui(ui, |ui| {
                    for (mi, m) in app.monitors.iter().enumerate() {
                        let w = m.rect.right - m.rect.left;
                        let h = m.rect.bottom - m.rect.top;
                        ui.selectable_value(
                            &mut app.edit_profile_mon_idx,
                            mi,
                            format!("Monitor {} ({}×{})", mi + 1, w, h),
                        );
                    }
                });
        });

        ui.label("Window process (optional):");
        ui.add(
            egui::TextEdit::singleline(&mut app.edit_profile_window_process)
                .hint_text("e.g. Diablo IV.exe — leave blank if not needed")
                .desired_width(f32::INFINITY),
        );

        ui.horizontal(|ui| {
            ui.checkbox(
                &mut app.edit_profile_force_primary,
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
                if let Some(idx) = app.editing_profile_idx {
                    let prof = &mut app.data.profiles[idx];
                    if let Some(new_exe) = app.edit_profile_exe.take() {
                        prof.name = new_exe.file_name().unwrap().to_string_lossy().into_owned();
                        prof.exe_path = new_exe;
                    }
                    let mon = &app.monitors[app.edit_profile_mon_idx];
                    prof.target_monitor_name = mon.device_name.clone();
                    prof.target_monitor_rect = Some(SerializableRect {
                        left: mon.rect.left,
                        top: mon.rect.top,
                        right: mon.rect.right,
                        bottom: mon.rect.bottom,
                    });
                    let proc = app.edit_profile_window_process.trim().to_string();
                    prof.window_process_name = if proc.is_empty() { None } else { Some(proc) };
                    prof.force_primary = app.edit_profile_force_primary;
                    app.save_data();
                }
                app.editing_profile_idx = None;
                app.edit_profile_exe = None;
                app.edit_profile_window_process.clear();
                app.edit_profile_force_primary = false;
            }
            if ui.button("✖ Cancel").clicked() {
                app.editing_profile_idx = None;
                app.edit_profile_exe = None;
                app.edit_profile_window_process.clear();
                app.edit_profile_force_primary = false;
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("🗑 Delete").clicked() {
                    *to_remove = Some(i);
                    app.editing_profile_idx = None;
                }
            });
        });
    });
}

fn draw_profile_card(
    app: &mut WindowManagerApp,
    ui: &mut egui::Ui,
    i: usize,
    p: &AppProfile,
    to_remove: &mut Option<usize>,
) {
    ui.group(|ui| {
        ui.horizontal(|ui| {
            ui.label(format!("🚀 {}", p.name));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("🗑 Delete").clicked() {
                    *to_remove = Some(i);
                }
                if ui.button("✏ Edit").clicked() {
                    app.editing_profile_idx = Some(i);
                    app.edit_profile_exe = None;
                    app.edit_profile_mon_idx = app
                        .monitors
                        .iter()
                        .position(|m| m.device_name == p.target_monitor_name)
                        .unwrap_or(0);
                    app.edit_profile_window_process =
                        p.window_process_name.clone().unwrap_or_default();
                    app.edit_profile_force_primary = p.force_primary;
                }
                if ui.button("▶ Launch").clicked() {
                    WindowManagerApp::launch_profile(p, Arc::clone(&app.status_message));
                }
            });
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

/// Draw the "New Profile" creation group.
pub fn draw_new_profile_form(app: &mut WindowManagerApp, ui: &mut egui::Ui) {
    ui.group(|ui| {
        ui.label("➕ New Profile:");
        ui.horizontal(|ui| {
            if ui.button("📁 Select EXE").clicked() {
                app.new_profile_exe = rfd::FileDialog::new()
                    .add_filter("Executable", &["exe"])
                    .pick_file();
            }
            if let Some(p) = &app.new_profile_exe {
                ui.label(
                    egui::RichText::new(p.file_name().unwrap().to_string_lossy())
                        .color(egui::Color32::LIGHT_GREEN),
                );
            } else {
                ui.label(egui::RichText::new("No file selected").color(egui::Color32::GRAY));
            }
        });

        ui.horizontal(|ui| {
            ui.label("Target monitor:");
            egui::ComboBox::from_id_salt("target_mon")
                .selected_text(if app.monitors.is_empty() {
                    "No monitors".to_string()
                } else {
                    format!("Monitor {}", app.selected_mon_idx + 1)
                })
                .show_ui(ui, |ui| {
                    for (i, m) in app.monitors.iter().enumerate() {
                        let w = m.rect.right - m.rect.left;
                        let h = m.rect.bottom - m.rect.top;
                        ui.selectable_value(
                            &mut app.selected_mon_idx,
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
                egui::TextEdit::singleline(&mut app.new_profile_window_process)
                    .hint_text("e.g. Diablo IV.exe  — leave blank if not needed")
                    .desired_width(f32::INFINITY),
            );
        });

        ui.horizontal(|ui| {
            ui.checkbox(
                &mut app.new_profile_force_primary,
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
                app.new_profile_exe.is_some() && !app.monitors.is_empty(),
                egui::Button::new("💾 Save Profile"),
            )
            .clicked()
        {
            let exe = app.new_profile_exe.clone().unwrap();
            let mon = &app.monitors[app.selected_mon_idx];
            let proc_name = app.new_profile_window_process.trim().to_string();
            app.data.profiles.push(crate::models::AppProfile {
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
                force_primary: app.new_profile_force_primary,
            });
            app.new_profile_exe = None;
            app.new_profile_window_process.clear();
            app.new_profile_force_primary = false;
            app.save_data();
        }
    });
}

/// Draw the status bar (separator + colored status label).
pub fn draw_status_bar(app: &WindowManagerApp, ui: &mut egui::Ui) {
    let status = app.status_message.lock().clone();
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
}
