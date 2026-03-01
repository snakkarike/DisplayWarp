use std::sync::Arc;

use eframe::egui;
use egui_phosphor::regular;

use crate::app::WindowManagerApp;
use crate::models::{AppProfile, SerializableRect};

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn truncate_text(text: &str, max_chars: usize) -> String {
    if text.chars().count() > max_chars {
        text.chars().take(max_chars - 1).collect::<String>() + "…"
    } else {
        text.to_string()
    }
}

// ─── Saved Profiles List ─────────────────────────────────────────────────────

pub fn draw_profiles_list(app: &mut WindowManagerApp, ui: &mut egui::Ui) {
    let mut to_remove: Option<usize> = None;
    let profiles: Vec<AppProfile> = app.data.lock().profiles.clone();

    if profiles.is_empty() {
        ui.label(
            egui::RichText::new("No profiles yet — create one on the left.")
                .small()
                .color(if app.dark_mode {
                    egui::Color32::GRAY
                } else {
                    egui::Color32::from_gray(100)
                }),
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

// ─── Profile Card ────────────────────────────────────────────────────────────

fn draw_profile_card(
    app: &mut WindowManagerApp,
    ui: &mut egui::Ui,
    i: usize,
    p: &AppProfile,
    to_remove: &mut Option<usize>,
) {
    egui::Frame::group(ui.style())
        .inner_margin(egui::Margin::same(12))
        .corner_radius(egui::CornerRadius::same(8))
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
            // ── Header: name + display badge (Vertical layout for narrow columns) ──
            ui.vertical(|ui| {
                ui.label(egui::RichText::new(&p.name).strong().size(13.0));
                ui.horizontal(|ui| {
                    let raw_mon = p
                        .target_monitor_name
                        .replace("\\\\.\\", "")
                        .replace("DISPLAY", "Display ");
                    let badge_text =
                        format!("{} {}", regular::MONITOR, truncate_text(&raw_mon, 15));
                    ui.label(
                        egui::RichText::new(badge_text)
                            .small()
                            .color(if app.dark_mode {
                                egui::Color32::from_rgb(150, 200, 255)
                            } else {
                                egui::Color32::from_rgb(37, 99, 235)
                            }),
                    );

                    if let Some(audio_id) = &p.target_audio_device_id {
                        let audio_name = app
                            .audio_devices
                            .iter()
                            .find(|d| d.id == *audio_id)
                            .map(|d| d.name.clone())
                            .unwrap_or_else(|| "Unknown Audio".to_string());

                        let badge_text = format!(
                            "{} {}",
                            regular::SPEAKER_HIGH,
                            truncate_text(&audio_name, 15)
                        );

                        ui.label(
                            egui::RichText::new(badge_text)
                                .small()
                                .color(egui::Color32::from_rgb(167, 139, 250)),
                        );
                    }
                });
            });

            // ── Exe path ──
            let exe_shown = truncate_text(&p.exe_path.display().to_string(), 45);
            ui.label(
                egui::RichText::new(format!("{} {}", regular::FOLDER_OPEN, exe_shown))
                    .small()
                    .color(if app.dark_mode {
                        egui::Color32::GRAY
                    } else {
                        egui::Color32::from_gray(80)
                    }),
            );

            // ── Window process name (if any) ──
            if let Some(proc) = &p.window_process_name {
                let proc_shown = truncate_text(proc, 30);
                ui.label(
                    egui::RichText::new(format!("{} {}", regular::FILE, proc_shown))
                        .small()
                        .color(if app.dark_mode {
                            egui::Color32::GRAY
                        } else {
                            egui::Color32::from_gray(80)
                        }),
                );
            }

            // ── Persistent toggle ──
            // ui.horizontal(|ui| {
            //     let mut persistent = p.persistent_monitor;
            //     if ui.checkbox(&mut persistent, "Persistent Window").changed() {
            //         app.data.lock().profiles[i].persistent_monitor = persistent;
            //         app.save_data();
            //     }
            // });

            ui.add_space(2.0);

            // ── Action buttons: Launch | Edit | Delete ──
            ui.horizontal(|ui| {
                let btn_width = (ui.available_width() - 16.0) / 3.0;

                if ui
                    .add_sized(
                        [btn_width, 24.0],
                        egui::Button::new(format!("{} Launch", regular::PLAY)),
                    )
                    .clicked()
                {
                    WindowManagerApp::launch_profile(
                        p,
                        Arc::clone(&app.status_message),
                        Arc::clone(&app.status_log),
                    );
                }
                if ui
                    .add_sized(
                        [btn_width, 24.0],
                        egui::Button::new(format!("{} Edit", regular::PENCIL_SIMPLE)),
                    )
                    .clicked()
                {
                    app.editing_profile_idx = Some(i);
                    app.edit_profile_name = p.name.clone();
                    app.edit_profile_exe = None;
                    app.edit_profile_mon_idx = app
                        .monitors
                        .iter()
                        .position(|m| m.device_name == p.target_monitor_name)
                        .unwrap_or(0);
                    app.edit_profile_window_process =
                        p.window_process_name.clone().unwrap_or_default();
                    app.edit_profile_audio_device_idx = p
                        .target_audio_device_id
                        .as_ref()
                        .and_then(|id| app.audio_devices.iter().position(|d| d.id == *id))
                        .map(|pos| pos + 1)
                        .unwrap_or(0);
                }
                if ui
                    .add_sized(
                        [btn_width, 24.0],
                        egui::Button::new(format!("{} Delete", regular::TRASH)),
                    )
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
        .inner_margin(egui::Margin::same(12))
        .corner_radius(egui::CornerRadius::same(8))
        .fill(if app.dark_mode {
            egui::Color32::from_rgb(45, 25, 25)
        } else {
            egui::Color32::from_rgb(254, 242, 242)
        })
        .stroke(egui::Stroke::new(
            1.5,
            if app.dark_mode {
                egui::Color32::from_rgb(239, 68, 68)
            } else {
                egui::Color32::from_rgb(220, 38, 38)
            },
        ))
        .show(ui, |ui| {
            ui.label(
                egui::RichText::new(format!("{} Editing Profile", regular::PENCIL_SIMPLE))
                    .color(if app.dark_mode {
                        egui::Color32::from_rgb(254, 202, 202)
                    } else {
                        egui::Color32::from_rgb(153, 27, 27)
                    })
                    .strong(),
            );

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
                        egui::RichText::new(format!("{} Profile Name", regular::PENCIL_SIMPLE))
                            .strong(),
                    );
                    ui.add_space(4.0);
                    ui.add(
                        egui::TextEdit::singleline(&mut app.edit_profile_name)
                            .desired_width(ui.available_width()),
                    );
                });

            ui.add_space(2.0);

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
                    ui.set_min_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.label(regular::FOLDER_OPEN);
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
                        let shown_text = truncate_text(&shown, 25);
                        ui.label(egui::RichText::new(shown_text).color(egui::Color32::LIGHT_GREEN));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui
                                .add(egui::Button::new(egui::RichText::new("Change").strong()))
                                .clicked()
                            {
                                app.edit_profile_exe = rfd::FileDialog::new()
                                    .add_filter("Executable", &["exe"])
                                    .pick_file();
                            }
                        });
                    });
                });

            ui.add_space(2.0);

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
                        egui::RichText::new(format!("{} Target monitor", regular::MONITOR))
                            .strong(),
                    );
                    ui.add_space(4.0);
                    egui::ComboBox::from_id_salt(format!("edit_mon_{i}"))
                        .selected_text(if app.monitors.is_empty() {
                            "No monitors".to_string()
                        } else {
                            format!("Monitor {}", app.edit_profile_mon_idx + 1)
                        })
                        .width(ui.available_width())
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

            ui.add_space(2.0);

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
                        egui::RichText::new(format!("{} Window process", regular::FILE)).strong(),
                    );
                    ui.add_space(4.0);
                    ui.add(
                        egui::TextEdit::singleline(&mut app.edit_profile_window_process)
                            .hint_text("e.g. Diablo IV.exe")
                            .desired_width(ui.available_width()),
                    );
                });

            ui.add_space(2.0);

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
                        egui::RichText::new(format!("{} Audio Output", regular::SPEAKER_HIGH))
                            .strong(),
                    );
                    ui.add_space(4.0);

                    let audio_text =
                        if app.audio_devices.is_empty() || app.edit_profile_audio_device_idx == 0 {
                            "Default (System)".to_string()
                        } else {
                            app.audio_devices
                                .get(app.edit_profile_audio_device_idx - 1)
                                .map(|d| truncate_text(&d.name, 25))
                                .unwrap_or_else(|| "Default (System)".to_string())
                        };

                    ui.horizontal(|ui| {
                        egui::ComboBox::from_id_salt(format!("edit_audio_{i}"))
                            .selected_text(audio_text)
                            .width(ui.available_width() - 60.0)
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut app.edit_profile_audio_device_idx,
                                    0,
                                    "Default (System)",
                                );
                                for (di, d) in app.audio_devices.iter().enumerate() {
                                    let item_text = truncate_text(&d.name, 40);
                                    ui.selectable_value(
                                        &mut app.edit_profile_audio_device_idx,
                                        di + 1,
                                        item_text,
                                    );
                                }
                            });

                        if ui
                            .add_enabled(
                                app.edit_profile_audio_device_idx > 0,
                                egui::Button::new("Test"),
                            )
                            .clicked()
                        {
                            if let Some(d) =
                                app.audio_devices.get(app.edit_profile_audio_device_idx - 1)
                            {
                                let id = d.id.clone();
                                std::thread::spawn(move || {
                                    let _ = crate::audio::play_test_beep(&id);
                                });
                            }
                        }
                    });
                });

            ui.add_space(4.0);

            ui.horizontal(|ui| {
                if ui
                    .add(egui::Button::new(format!("{} Save", regular::CHECK)))
                    .clicked()
                {
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
                        prof.name = app.edit_profile_name.trim().to_string();
                        if let Some(new_exe) = app.edit_profile_exe.take() {
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
                        prof.target_audio_device_id = if app.edit_profile_audio_device_idx > 0
                            && (app.edit_profile_audio_device_idx - 1) < app.audio_devices.len()
                        {
                            Some(
                                app.audio_devices[app.edit_profile_audio_device_idx - 1]
                                    .id
                                    .clone(),
                            )
                        } else {
                            None
                        };
                        drop(data);
                        app.save_data();
                    }
                    app.editing_profile_idx = None;
                    app.edit_profile_exe = None;
                    app.edit_profile_window_process.clear();
                }
                if ui
                    .add(egui::Button::new(format!("{} Cancel", regular::X)))
                    .clicked()
                {
                    app.editing_profile_idx = None;
                    app.edit_profile_exe = None;
                    app.edit_profile_window_process.clear();
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add(egui::Button::new(format!("{} Delete", regular::TRASH)))
                        .clicked()
                    {
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
            ui.set_min_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(regular::FOLDER_OPEN);
                if let Some(p) = &app.new_profile_exe {
                    let filename = p.file_name().unwrap().to_string_lossy().into_owned();
                    let shown = truncate_text(&filename, 25);
                    ui.label(egui::RichText::new(shown).color(egui::Color32::LIGHT_GREEN));
                } else {
                    ui.label(
                        egui::RichText::new("No File Selected")
                            .color(egui::Color32::GRAY)
                            .strong(),
                    );
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .button(egui::RichText::new("Select EXE").strong())
                        .clicked()
                    {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("Executable", &["exe"])
                            .pick_file()
                        {
                            app.new_profile_exe = Some(path.clone());
                            if app.new_profile_name.is_empty() {
                                app.new_profile_name =
                                    path.file_name().unwrap().to_string_lossy().into_owned();
                            }
                        }
                    }
                });
            });
        });

    ui.add_space(2.0);

    // Profile Name
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
                egui::RichText::new(format!("{} Profile Name", regular::PENCIL_SIMPLE)).strong(),
            );
            ui.add_space(4.0);
            ui.add(
                egui::TextEdit::singleline(&mut app.new_profile_name)
                    .hint_text("Enter profile name")
                    .desired_width(ui.available_width()),
            );
        });

    ui.add_space(2.0);

    // Monitor selector
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
                egui::RichText::new(format!("{} Select Preferred Monitor", regular::MONITOR))
                    .strong(),
            );
            ui.add_space(4.0);
            egui::ComboBox::from_id_salt("new_target_mon")
                .selected_text(if app.monitors.is_empty() {
                    "No monitors".to_string()
                } else {
                    format!("Monitor {}", app.selected_mon_idx + 1)
                })
                .width(ui.available_width())
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

    ui.add_space(2.0);

    // Window process
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
            ui.add(
                egui::TextEdit::multiline(&mut app.new_profile_window_process)
                    .hint_text("e.g. Diablo IV.exe - Leave blank if not needed.")
                    .desired_width(ui.available_width())
                    .desired_rows(2),
            );
        });

    ui.add_space(2.0);

    // New Audio selector
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
                egui::RichText::new(format!("{} Audio Output", regular::SPEAKER_HIGH)).strong(),
            );
            ui.add_space(4.0);

            let audio_text =
                if app.audio_devices.is_empty() || app.new_profile_audio_device_idx == 0 {
                    "Default (System)".to_string()
                } else {
                    app.audio_devices
                        .get(app.new_profile_audio_device_idx - 1)
                        .map(|d| truncate_text(&d.name, 25))
                        .unwrap_or_else(|| "Default (System)".to_string())
                };

            ui.horizontal(|ui| {
                egui::ComboBox::from_id_salt("new_audio_switch")
                    .selected_text(audio_text)
                    .width(ui.available_width() - 60.0)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut app.new_profile_audio_device_idx,
                            0,
                            "Default (System)",
                        );
                        for (di, d) in app.audio_devices.iter().enumerate() {
                            let item_text = truncate_text(&d.name, 40);
                            ui.selectable_value(
                                &mut app.new_profile_audio_device_idx,
                                di + 1,
                                item_text,
                            );
                        }
                    });

                if ui
                    .add_enabled(
                        app.new_profile_audio_device_idx > 0,
                        egui::Button::new("Test"),
                    )
                    .clicked()
                {
                    if let Some(d) = app.audio_devices.get(app.new_profile_audio_device_idx - 1) {
                        let id = d.id.clone();
                        std::thread::spawn(move || {
                            let _ = crate::audio::play_test_beep(&id);
                        });
                    }
                }
            });
        });

    ui.add_space(8.0);

    if ui
        .add_sized(
            [ui.available_width(), 35.0],
            egui::Button::new(
                egui::RichText::new(format!("{} Create Profile", regular::PLUS)).strong(),
            ),
        )
        .clicked()
    {
        if app.new_profile_exe.is_some() && !app.monitors.is_empty() {
            let pid_mon = if let Some(mon) = app.monitors.get(app.selected_mon_idx) {
                mon
            } else if let Some(first_mon) = app.monitors.first() {
                first_mon
            } else {
                return;
            };

            let mut data = app.data.lock();
            let proc = app.new_profile_window_process.trim().to_string();
            data.profiles.push(AppProfile {
                name: app.new_profile_name.trim().to_string(),
                exe_path: app.new_profile_exe.clone().unwrap(),
                target_monitor_name: pid_mon.device_name.clone(),
                target_monitor_rect: Some(SerializableRect {
                    left: pid_mon.rect.left,
                    top: pid_mon.rect.top,
                    right: pid_mon.rect.right,
                    bottom: pid_mon.rect.bottom,
                }),
                window_process_name: if proc.is_empty() { None } else { Some(proc) },
                force_primary: false,
                persistent_monitor: false,
                target_audio_device_id: if app.new_profile_audio_device_idx > 0
                    && (app.new_profile_audio_device_idx - 1) < app.audio_devices.len()
                {
                    Some(
                        app.audio_devices[app.new_profile_audio_device_idx - 1]
                            .id
                            .clone(),
                    )
                } else {
                    None
                },
            });
            drop(data);

            app.new_profile_exe = None;
            app.new_profile_name.clear();
            app.new_profile_window_process.clear();
            app.new_profile_audio_device_idx = 0;
            app.save_data();
            *app.status_message.lock() = "✅ Profile created.".into();
        }
    }
}

// ─── Move Live Window ────────────────────────────────────────────────────────

pub fn draw_live_process_mover(app: &mut WindowManagerApp, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(format!("{} Move Live Window", regular::MONITOR))
                .size(14.0)
                .strong(),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .add(egui::Button::new(format!(
                    "{} Refresh",
                    regular::ARROW_CLOCKWISE
                )))
                .clicked()
            {
                app.refresh_live_processes();
            }
        });
    });

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
                        ui.selectable_value(
                            &mut app.live_move_mon_idx,
                            i,
                            format!("Monitor {} ({}×{})", i + 1, w, h),
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
}

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

// ─── Display Layout Tab ──────────────────────────────────────────────────────

pub fn draw_display_tab(app: &mut WindowManagerApp, ui: &mut egui::Ui) {
    if app.monitors.is_empty() {
        ui.vertical_centered(|ui| {
            ui.add_space(100.0);
            ui.label(
                egui::RichText::new("No monitors detected.")
                    .size(20.0)
                    .color(egui::Color32::GRAY),
            );
        });
        return;
    }

    ui.columns(2, |cols| {
        // Col 1: Interactive Canvas & Save Form
        cols[0].vertical(|ui| {
            ui.set_min_height(500.0);
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
                    ui.label(
                        egui::RichText::new(format!("{} Arrange Monitors", regular::CROP))
                            .size(16.0)
                            .strong(),
                    );
                    ui.add_space(8.0);

                    // Interactive Dragging Canvas
                    let (response, painter) = ui.allocate_painter(
                        egui::vec2(ui.available_width(), 320.0),
                        egui::Sense::drag(),
                    );
                    let rect = response.rect;

                    painter.rect_filled(
                        rect,
                        4.0,
                        if app.dark_mode {
                            egui::Color32::from_rgb(15, 15, 15)
                        } else {
                            egui::Color32::from_rgb(230, 235, 240)
                        },
                    );

                    let min_x = app.monitors.iter().map(|m| m.rect.left).min().unwrap_or(0);
                    let max_x = app.monitors.iter().map(|m| m.rect.right).max().unwrap_or(1);
                    let min_y = app.monitors.iter().map(|m| m.rect.top).min().unwrap_or(0);
                    let max_y = app
                        .monitors
                        .iter()
                        .map(|m| m.rect.bottom)
                        .max()
                        .unwrap_or(1);

                    let width = (max_x - min_x) as f32;
                    let height = (max_y - min_y) as f32;
                    let scale = (rect.width() / width).min(rect.height() / height) * 0.85;
                    let center = rect.center();

                    // Handle Dragging Logic
                    if response.drag_started() {
                        if let Some(pointer_pos) = response.interact_pointer_pos() {
                            for (i, m) in app.monitors.iter().enumerate() {
                                let m_rect = egui::Rect::from_min_max(
                                    center
                                        + egui::vec2(
                                            (m.rect.left - min_x) as f32 * scale
                                                - (width * scale / 2.0),
                                            (m.rect.top - min_y) as f32 * scale
                                                - (height * scale / 2.0),
                                        ),
                                    center
                                        + egui::vec2(
                                            (m.rect.right - min_x) as f32 * scale
                                                - (width * scale / 2.0),
                                            (m.rect.bottom - min_y) as f32 * scale
                                                - (height * scale / 2.0),
                                        ),
                                );
                                if m_rect.contains(pointer_pos) {
                                    app.dragging_monitor_idx = Some(i);
                                    app.drag_start_pos = Some(pointer_pos);
                                    app.original_monitor_rect = Some(m.rect.clone());
                                    break;
                                }
                            }
                        }
                    }

                    if response.dragged() {
                        if let (Some(idx), Some(start_pos), Some(orig_rect)) = (
                            app.dragging_monitor_idx,
                            app.drag_start_pos,
                            &app.original_monitor_rect,
                        ) {
                            if let Some(current_pos) = response.interact_pointer_pos() {
                                let delta = current_pos - start_pos;
                                let delta_virtual_x = (delta.x / scale) as i32;
                                let delta_virtual_y = (delta.y / scale) as i32;

                                let width = orig_rect.right - orig_rect.left;
                                let height = orig_rect.bottom - orig_rect.top;

                                app.monitors[idx].rect.left = orig_rect.left + delta_virtual_x;
                                app.monitors[idx].rect.top = orig_rect.top + delta_virtual_y;
                                app.monitors[idx].rect.right = app.monitors[idx].rect.left + width;
                                app.monitors[idx].rect.bottom = app.monitors[idx].rect.top + height;

                                // Snap primary monitor loosely to 0,0 grid if close
                                if (app.monitors[idx].rect.left).abs() < 50
                                    && (app.monitors[idx].rect.top).abs() < 50
                                {
                                    app.monitors[idx].rect.left = 0;
                                    app.monitors[idx].rect.top = 0;
                                    app.monitors[idx].rect.right = width;
                                    app.monitors[idx].rect.bottom = height;
                                }
                            }
                        }
                    }

                    if response.drag_stopped() {
                        app.dragging_monitor_idx = None;
                        app.drag_start_pos = None;
                        app.original_monitor_rect = None;
                    }

                    // Draw Rects
                    for (i, m) in app.monitors.iter().enumerate() {
                        let is_primary = m.rect.left == 0 && m.rect.top == 0;
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

                        let fill = if is_primary {
                            if app.dark_mode {
                                egui::Color32::from_rgb(76, 29, 149)
                            } else {
                                egui::Color32::from_rgb(139, 92, 246)
                            }
                        } else {
                            if app.dark_mode {
                                egui::Color32::from_rgb(30, 41, 59)
                            } else {
                                egui::Color32::from_rgb(148, 163, 184)
                            }
                        };

                        painter.rect_filled(m_rect, 4.0, fill);
                        painter.rect_stroke(
                            m_rect,
                            egui::CornerRadius::same(4),
                            egui::Stroke::new(1.5, egui::Color32::from_white_alpha(40)),
                            egui::StrokeKind::Middle,
                        );

                        painter.text(
                            m_rect.center(),
                            egui::Align2::CENTER_CENTER,
                            format!("Monitor {}", i + 1).to_owned(),
                            egui::FontId::proportional(14.0),
                            egui::Color32::WHITE,
                        );
                        painter.text(
                            m_rect.left_top() + egui::vec2(6.0, 6.0),
                            egui::Align2::LEFT_TOP,
                            format!("{}, {}", m.rect.left, m.rect.top).to_owned(),
                            egui::FontId::proportional(10.0),
                            egui::Color32::from_white_alpha(150),
                        );
                    }

                    ui.add_space(12.0);
                    ui.separator();
                    ui.add_space(8.0);

                    // Save Form
                    ui.label(egui::RichText::new("Save Layout").strong());
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut app.new_display_profile_name)
                                .hint_text("Layout Name...")
                                .desired_width(ui.available_width() - 100.0),
                        );
                        if ui
                            .add_sized(
                                [90.0, 24.0],
                                egui::Button::new(format!("{} Save", regular::FLOPPY_DISK)),
                            )
                            .clicked()
                        {
                            if !app.new_display_profile_name.is_empty() {
                                let monitors_snapshot = app
                                    .monitors
                                    .iter()
                                    .map(|m| crate::models::SavedMonitorPos {
                                        device_name: m.device_name.clone(),
                                        rect: crate::models::SerializableRect {
                                            left: m.rect.left,
                                            top: m.rect.top,
                                            right: m.rect.right,
                                            bottom: m.rect.bottom,
                                        },
                                    })
                                    .collect();

                                app.data.lock().display_profiles.push(
                                    crate::models::SavedDisplayLayout {
                                        name: app.new_display_profile_name.clone(),
                                        monitors: monitors_snapshot,
                                    },
                                );
                                app.new_display_profile_name.clear();
                                app.save_data();
                                WindowManagerApp::push_status(
                                    &app.status_message,
                                    &app.status_log,
                                    "✅ Display layout saved.",
                                );
                            }
                        }
                    });
                });
        });

        // Col 2: Saved Display Profiles List
        cols[1].vertical(|ui| {
            ui.set_min_height(500.0);
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
                    ui.label(
                        egui::RichText::new(format!("{} Saved Profiles", regular::BOOKMARK_SIMPLE))
                            .size(16.0)
                            .strong(),
                    );
                    ui.add_space(8.0);

                    let mut to_remove = None;
                    let display_profiles = app.data.lock().display_profiles.clone();

                    if display_profiles.is_empty() {
                        ui.label(
                            egui::RichText::new("No display profiles saved.")
                                .color(egui::Color32::GRAY),
                        );
                    }

                    egui::ScrollArea::vertical()
                        .id_salt("display_profiles_scroll")
                        .show(ui, |ui| {
                            for (i, p) in display_profiles.iter().enumerate() {
                                egui::Frame::group(ui.style())
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
                                            egui::Color32::from_rgb(50, 50, 50)
                                        } else {
                                            egui::Color32::from_rgb(226, 232, 240)
                                        },
                                    ))
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            ui.label(egui::RichText::new(&p.name).strong());
                                            ui.with_layout(
                                                egui::Layout::right_to_left(egui::Align::Center),
                                                |ui| {
                                                    if ui
                                                        .button(format!("{} Apply", regular::PLAY))
                                                        .clicked()
                                                    {
                                                        crate::monitor::restore_monitor_layout(
                                                            &p.monitors,
                                                        );
                                                        WindowManagerApp::push_status(
                                                            &app.status_message,
                                                            &app.status_log,
                                                            format!("✅ Applied '{}'", p.name),
                                                        );
                                                        app.refresh_monitors();
                                                    }
                                                    if ui
                                                        .button(format!(
                                                            "{} Delete",
                                                            regular::TRASH
                                                        ))
                                                        .clicked()
                                                    {
                                                        to_remove = Some(i);
                                                    }
                                                },
                                            );
                                        });
                                    });
                                ui.add_space(4.0);
                            }
                        });

                    if let Some(idx) = to_remove {
                        app.data.lock().display_profiles.remove(idx);
                        app.save_data();
                    }
                });
        });
    });
}
