use std::sync::Arc;

use eframe::egui;

use crate::app::WindowManagerApp;
use crate::models::{AppProfile, SerializableRect};

// ─── Saved Profiles List ─────────────────────────────────────────────────────

pub fn draw_profiles_list(app: &mut WindowManagerApp, ui: &mut egui::Ui) {
    let mut to_remove: Option<usize> = None;
    let profiles: Vec<AppProfile> = app.data.lock().profiles.clone();

    if profiles.is_empty() {
        ui.label(
            egui::RichText::new("No profiles yet — create one on the left.")
                .small()
                .color(egui::Color32::GRAY),
        );
    }

    for (i, p) in profiles.iter().enumerate() {
        let is_editing = app.editing_profile_idx == Some(i);
        if is_editing {
            draw_edit_profile_form(app, ui, i, p, &mut to_remove);
        } else {
            draw_profile_card(app, ui, i, p, &mut to_remove);
        }
    }
    if let Some(i) = to_remove {
        app.data.lock().profiles.remove(i);
        app.save_data();
    }
}

// ─── Profile Card (mockup style) ─────────────────────────────────────────────

fn draw_profile_card(
    app: &mut WindowManagerApp,
    ui: &mut egui::Ui,
    i: usize,
    p: &AppProfile,
    to_remove: &mut Option<usize>,
) {
    egui::Frame::group(ui.style())
        .inner_margin(egui::Margin::same(8.0))
        .rounding(egui::Rounding::same(6.0))
        .show(ui, |ui| {
            // ── Header: name + display badge ──
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(&p.name).strong().size(13.0));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Display badge
                    let badge_text = format!(
                        "📺 {}",
                        p.target_monitor_name
                            .replace("\\\\.\\", "")
                            .replace("DISPLAY", "Display ")
                    );
                    ui.label(
                        egui::RichText::new(badge_text)
                            .small()
                            .color(egui::Color32::from_rgb(150, 200, 255)),
                    );
                });
            });

            // ── Exe path ──
            ui.label(
                egui::RichText::new(format!("📁 {}", p.exe_path.display()))
                    .small()
                    .color(egui::Color32::GRAY),
            );

            // ── Window process name (if any) ──
            if let Some(proc) = &p.window_process_name {
                ui.label(
                    egui::RichText::new(format!("📎 {}", proc))
                        .small()
                        .color(egui::Color32::GRAY),
                );
            }

            // ── Persistent toggle ──
            ui.horizontal(|ui| {
                let mut persistent = p.persistent_monitor;
                if ui.checkbox(&mut persistent, "Persistent Window").changed() {
                    app.data.lock().profiles[i].persistent_monitor = persistent;
                    app.save_data();
                }
            });

            ui.add_space(2.0);

            // ── Action buttons: Launch | Edit | Delete ──
            ui.horizontal(|ui| {
                let btn_width = (ui.available_width() - 16.0) / 3.0;

                if ui
                    .add_sized([btn_width, 24.0], egui::Button::new("▷ Launch"))
                    .clicked()
                {
                    WindowManagerApp::launch_profile(p, Arc::clone(&app.status_message));
                }
                if ui
                    .add_sized([btn_width, 24.0], egui::Button::new("✏ Edit"))
                    .clicked()
                {
                    app.editing_profile_idx = Some(i);
                    app.edit_profile_exe = None;
                    app.edit_profile_mon_idx = app
                        .monitors
                        .iter()
                        .position(|m| m.device_name == p.target_monitor_name)
                        .unwrap_or(0);
                    app.edit_profile_window_process =
                        p.window_process_name.clone().unwrap_or_default();
                }
                if ui
                    .add_sized([btn_width, 24.0], egui::Button::new("🗑 Delete"))
                    .clicked()
                {
                    *to_remove = Some(i);
                }
            });
        });

    ui.add_space(4.0);
}

// ─── Edit Profile Form ──────────────────────────────────────────────────────

fn draw_edit_profile_form(
    app: &mut WindowManagerApp,
    ui: &mut egui::Ui,
    i: usize,
    p: &AppProfile,
    to_remove: &mut Option<usize>,
) {
    egui::Frame::group(ui.style())
        .inner_margin(egui::Margin::same(8.0))
        .rounding(egui::Rounding::same(6.0))
        .stroke(egui::Stroke::new(1.5, egui::Color32::YELLOW))
        .show(ui, |ui| {
            ui.label(
                egui::RichText::new(format!("✏ Editing: {}", p.name))
                    .color(egui::Color32::YELLOW)
                    .strong(),
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
                    .hint_text("e.g. Diablo IV.exe")
                    .desired_width(f32::INFINITY),
            );

            ui.horizontal(|ui| {
                if ui.button("✅ Save").clicked() {
                    if let Some(idx) = app.editing_profile_idx {
                        if app.edit_profile_mon_idx >= app.monitors.len() {
                            app.edit_profile_mon_idx = 0;
                        }
                        if app.monitors.is_empty() {
                            app.editing_profile_idx = None;
                            app.edit_profile_exe = None;
                            app.edit_profile_window_process.clear();
                            return;
                        }
                        let mut data = app.data.lock();
                        let prof = &mut data.profiles[idx];
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
                        drop(data);
                        app.save_data();
                    }
                    app.editing_profile_idx = None;
                    app.edit_profile_exe = None;
                    app.edit_profile_window_process.clear();
                }
                if ui.button("✖ Cancel").clicked() {
                    app.editing_profile_idx = None;
                    app.edit_profile_exe = None;
                    app.edit_profile_window_process.clear();
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("🗑 Delete").clicked() {
                        *to_remove = Some(i);
                        app.editing_profile_idx = None;
                    }
                });
            });
        });

    ui.add_space(4.0);
}

// ─── New Profile Form ────────────────────────────────────────────────────────

pub fn draw_new_profile_form(app: &mut WindowManagerApp, ui: &mut egui::Ui) {
    // File selector
    ui.horizontal(|ui| {
        ui.label("📁");
        if let Some(p) = &app.new_profile_exe {
            ui.label(
                egui::RichText::new(p.file_name().unwrap().to_string_lossy())
                    .color(egui::Color32::LIGHT_GREEN),
            );
        } else {
            ui.label(egui::RichText::new("No File Selected").color(egui::Color32::GRAY));
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("Select EXE").clicked() {
                app.new_profile_exe = rfd::FileDialog::new()
                    .add_filter("Executable", &["exe"])
                    .pick_file();
            }
        });
    });

    ui.add_space(2.0);

    // Monitor selector
    ui.horizontal(|ui| {
        ui.label("📺 Select Preferred Monitor");
    });
    egui::ComboBox::from_id_salt("new_target_mon")
        .selected_text(if app.monitors.is_empty() {
            "No monitors".to_string()
        } else {
            format!("Monitor {}", app.selected_mon_idx + 1)
        })
        .width(ui.available_width() - 8.0)
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

    ui.add_space(2.0);

    // Window process
    ui.label("📎 Window Process");
    ui.add(
        egui::TextEdit::multiline(&mut app.new_profile_window_process)
            .hint_text("e.g. Diablo IV.exe - Leave blank if not needed.")
            .desired_width(f32::INFINITY)
            .desired_rows(2),
    );

    ui.add_space(4.0);

    // Save button
    let can_save = app.new_profile_exe.is_some() && !app.monitors.is_empty();
    if ui
        .add_sized(
            [ui.available_width(), 28.0],
            egui::Button::new("💾 Save Profile"),
        )
        .clicked()
        && can_save
    {
        let exe = app.new_profile_exe.clone().unwrap();
        let mon = &app.monitors[app.selected_mon_idx];
        let proc_name = app.new_profile_window_process.trim().to_string();
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
            window_process_name: if proc_name.is_empty() {
                None
            } else {
                Some(proc_name)
            },
            force_primary: false,
            persistent_monitor: false,
        });
        app.new_profile_exe = None;
        app.new_profile_window_process.clear();
        app.save_data();
    }
}

// ─── Move Live Window ────────────────────────────────────────────────────────

pub fn draw_live_process_mover(app: &mut WindowManagerApp, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("🖥 Move Live Window")
                .size(14.0)
                .strong(),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("🔄 Refresh Process List").clicked() {
                app.refresh_live_processes();
            }
        });
    });

    ui.add_space(4.0);

    // Window process selector
    ui.label("📎 Window Process");
    let current_label = app
        .live_processes
        .get(app.selected_live_process_idx)
        .map(|e| e.label.as_str())
        .unwrap_or("Select Live Process");
    egui::ComboBox::from_id_salt("live_proc")
        .selected_text(current_label)
        .width(ui.available_width() - 8.0)
        .show_ui(ui, |ui| {
            for (i, entry) in app.live_processes.iter().enumerate() {
                ui.selectable_value(&mut app.selected_live_process_idx, i, &entry.label);
            }
        });

    ui.add_space(4.0);

    // Target monitor selector
    ui.label("📺 Select Target Monitor");
    egui::ComboBox::from_id_salt("live_mon")
        .selected_text(if app.monitors.is_empty() {
            "No monitors".to_string()
        } else {
            format!("Monitor {}", app.live_move_mon_idx + 1)
        })
        .width(ui.available_width() - 8.0)
        .show_ui(ui, |ui| {
            for (i, m) in app.monitors.iter().enumerate() {
                let w = m.rect.right - m.rect.left;
                let h = m.rect.bottom - m.rect.top;
                ui.selectable_value(
                    &mut app.live_move_mon_idx,
                    i,
                    format!("Monitor {} ({}×{})", i + 1, w, h),
                );
            }
        });

    ui.add_space(6.0);

    // Move button
    let can_move = !app.live_processes.is_empty() && !app.monitors.is_empty();
    if ui
        .add_sized(
            [ui.available_width(), 30.0],
            egui::Button::new(egui::RichText::new("📦 Move Process").strong())
                .fill(egui::Color32::from_rgb(50, 200, 100)),
        )
        .clicked()
        && can_move
    {
        if let Some(entry) = app.live_processes.get(app.selected_live_process_idx) {
            let hwnd = entry.hwnd;
            let target = app.monitors[app.live_move_mon_idx].rect;
            WindowManagerApp::move_live_window(hwnd, target, Arc::clone(&app.status_message));
        }
    }
}

// ─── Status / Log Bar ────────────────────────────────────────────────────────

pub fn draw_status_bar(app: &WindowManagerApp, ui: &mut egui::Ui) {
    let status = app.status_message.lock().clone();
    let color = if status.starts_with('✅') {
        egui::Color32::LIGHT_GREEN
    } else if status.starts_with('❌') {
        egui::Color32::LIGHT_RED
    } else if status.starts_with('⚠') {
        egui::Color32::YELLOW
    } else {
        egui::Color32::GRAY
    };
    egui::Frame::none()
        .inner_margin(egui::Margin::same(6.0))
        .rounding(egui::Rounding::same(4.0))
        .fill(egui::Color32::from_gray(30))
        .show(ui, |ui| {
            ui.label(egui::RichText::new(&status).small().color(color));
        });
}
