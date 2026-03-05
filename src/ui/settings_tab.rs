use eframe::egui;
use egui_phosphor::regular;

use crate::app::WindowManagerApp;

pub fn draw_settings_tab(app: &mut WindowManagerApp, ui: &mut egui::Ui) {
    ui.columns(2, |cols| {
        let col_height = 400.0;
        // Col 1: Settings
        cols[0].vertical(|ui| {
            ui.set_min_height(col_height);
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
                    ui.set_width(ui.available_width());
                    ui.set_min_height(ui.available_height());
                    ui.label(
                        egui::RichText::new(format!("{} Settings", regular::GEAR))
                            .size(16.0)
                            .strong(),
                    );
                    ui.label(
                        egui::RichText::new("Configure DisplayWarp behavior and preferences.")
                            .small()
                            .color(if app.dark_mode {
                                egui::Color32::from_gray(140)
                            } else {
                                egui::Color32::from_gray(100)
                            }),
                    );
                    ui.add_space(16.0);

                    // Config Location Card
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
                            ui.set_width(ui.available_width());
                            ui.label(egui::RichText::new("Config Location").strong());
                            ui.add_space(8.0);

                            let config_path = crate::app::WindowManagerApp::get_config_path();

                            egui::ScrollArea::horizontal()
                                .id_salt("config_path_scroll")
                                .show(ui, |ui| {
                                    ui.label(
                                        egui::RichText::new(config_path.to_string_lossy().to_string())
                                            .code()
                                            .color(if app.dark_mode {
                                                egui::Color32::LIGHT_GRAY
                                            } else {
                                                egui::Color32::DARK_GRAY
                                            }),
                                    );
                                });

                            ui.add_space(4.0);

                            ui.horizontal(|ui| {
                                if ui.button(format!("{} Open Folder", regular::FOLDER_OPEN)).clicked() {
                                    if let Some(parent) = config_path.parent() {
                                        let _ = std::process::Command::new("explorer").arg(parent).spawn();
                                    }
                                }
                                if ui.button(format!("{} Change", regular::PENCIL_SIMPLE)).clicked() {
                                    if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                                        let exe_dir = std::env::current_exe()
                                            .unwrap_or_else(|_| std::path::PathBuf::from("."))
                                            .parent()
                                            .unwrap_or_else(|| std::path::Path::new("."))
                                            .to_path_buf();

                                        let new_config_file = folder.join("monitor_config.json");

                                        if config_path.exists() && !new_config_file.exists() {
                                            let _ = std::fs::copy(&config_path, &new_config_file);
                                        }

                                        let _ = std::fs::write(
                                            exe_dir.join("config_location.txt"),
                                            folder.to_string_lossy().as_ref(),
                                        );

                                        crate::app::WindowManagerApp::push_status(
                                            &app.status_message,
                                            &app.status_log,
                                            "📁 Config location updated.",
                                        );
                                    }
                                }
                            });
                        });

                    ui.add_space(8.0);

                    // Theme Card
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
                            ui.set_width(ui.available_width());
                            ui.horizontal(|ui| {
                                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                                    ui.label(egui::RichText::new("Theme:").strong());
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        let theme_icon = if app.dark_mode { regular::SUN } else { regular::MOON };
                                        if ui.button(format!("{} Toggle Theme", theme_icon)).clicked() {
                                            app.dark_mode = !app.dark_mode;
                                        }
                                    });
                                });
                            });
                        });

                    ui.add_space(8.0);

                    // Behavior Card
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
                            ui.set_width(ui.available_width());
                            ui.label(egui::RichText::new("Application Behavior").strong());
                            ui.add_space(8.0);

                            let mut data = app.data.lock();
                            let mut dirty = false;

                            if ui
                                .checkbox(&mut data.close_to_tray, "Minimize to tray on close")
                                .clicked()
                            {
                                dirty = true;
                            }

                            ui.add_space(4.0);
                            if ui
                                .checkbox(&mut data.start_minimized, "Start minimized to tray")
                                .clicked()
                            {
                                dirty = true;
                            }

                            ui.add_space(4.0);
                            if ui
                                .checkbox(&mut data.start_on_boot, "Start on system startup")
                                .clicked()
                            {
                                dirty = true;
                                let al = WindowManagerApp::get_auto_launch();
                                if data.start_on_boot {
                                    let _ = al.enable();
                                } else {
                                    let _ = al.disable();
                                }
                            }

                            if dirty {
                                drop(data);
                                app.save_data();
                                crate::app::WindowManagerApp::push_status(
                                    &app.status_message,
                                    &app.status_log,
                                    "⚙️ Application behavior settings updated.",
                                );
                            }
                        });

                    ui.add_space(8.0);

                    // Watcher Interval Card
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
                            ui.set_width(ui.available_width());
                            ui.label(egui::RichText::new("Background Watcher Interval").strong());
                            ui.add_space(4.0);

                            let mut data = app.data.lock();
                            let mut dirty = false;
                            let mut interval = data.watcher_interval_secs;
                            if ui
                                .add(egui::Slider::new(&mut interval, 1..=30).suffix(" seconds"))
                                .changed()
                            {
                                data.watcher_interval_secs = interval;
                                dirty = true;
                            }

                            if dirty {
                                drop(data);
                                app.save_data();
                                crate::app::WindowManagerApp::push_status(
                                    &app.status_message,
                                    &app.status_log,
                                    format!("⚙️ Watcher interval set to {}s.", interval),
                                );
                            }

                            ui.add_space(6.0);
                            ui.label(
                                egui::RichText::new("Controls how often DisplayWarp scans application placement to enforce persistence rules. Lower speeds save CPU.")
                                    .size(11.0)
                                    .color(egui::Color32::from_gray(140)),
                            );
                        });

                    ui.add_space(8.0);

                    // Profile Import/Export Card
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
                            ui.set_width(ui.available_width());
                            ui.label(egui::RichText::new(format!("{} Profile Backup", regular::ARCHIVE)).strong());
                            ui.add_space(4.0);
                            ui.label(
                                egui::RichText::new("Export or import all profiles as JSON.")
                                    .small()
                                    .color(egui::Color32::from_gray(140)),
                            );
                            ui.add_space(8.0);
                            ui.horizontal(|ui| {
                                if ui.button(format!("{} Export", regular::EXPORT)).clicked() {
                                    let profiles_json = {
                                        let d = app.data.lock();
                                        serde_json::to_string_pretty(&d.profiles).unwrap_or_default()
                                    };
                                    if let Some(path) = rfd::FileDialog::new()
                                        .set_file_name("displaywarp_profiles.json")
                                        .add_filter("JSON", &["json"])
                                        .save_file()
                                    {
                                        let ok = std::fs::write(&path, &profiles_json).is_ok();
                                        crate::app::WindowManagerApp::push_status(
                                            &app.status_message,
                                            &app.status_log,
                                            if ok { "📦 Profiles exported." } else { "❌ Export failed." },
                                        );
                                    }
                                }
                                if ui.button(format!("{} Import", regular::DOWNLOAD_SIMPLE)).clicked() {
                                    if let Some(path) = rfd::FileDialog::new()
                                        .add_filter("JSON", &["json"])
                                        .pick_file()
                                    {
                                        if let Ok(bytes) = std::fs::read(&path) {
                                            if let Ok(imported) = serde_json::from_slice::<Vec<crate::models::AppProfile>>(&bytes) {
                                                let mut d = app.data.lock();
                                                let names: std::collections::HashSet<String> =
                                                    d.profiles.iter().map(|p| p.name.clone()).collect();
                                                let mut added = 0usize;
                                                for p in imported {
                                                    if !names.contains(&p.name) {
                                                        d.profiles.push(p);
                                                        added += 1;
                                                    }
                                                }
                                                drop(d);
                                                app.save_data();
                                                crate::app::WindowManagerApp::push_status(
                                                    &app.status_message,
                                                    &app.status_log,
                                                    format!("📦 Imported {added} profile(s)."),
                                                );
                                            } else {
                                                crate::app::WindowManagerApp::push_status(
                                                    &app.status_message,
                                                    &app.status_log,
                                                    "❌ Import failed — invalid format.",
                                                );
                                            }
                                        }
                                    }
                                }
                            });
                        });
                });
        });

        // Col 2: About
        cols[1].vertical(|ui| {
            ui.set_min_height(col_height);
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
                    ui.set_width(ui.available_width());
                    ui.set_min_height(ui.available_height());
                    ui.label(
                        egui::RichText::new(format!("{} About DisplayWarp", regular::INFO))
                            .size(16.0)
                            .strong(),
                    );
                    ui.add_space(8.0);

                    if app.dark_mode {
                        ui.add(
                            egui::Image::new(egui::include_image!("../../assets/DisplayWarpLogoWhite.png"))
                                .max_width(200.0)
                                .maintain_aspect_ratio(true),
                        );
                    } else {
                        ui.add(
                            egui::Image::new(egui::include_image!("../../assets/DisplayWarpLogoBlack.png"))
                                .max_width(200.0)
                                .maintain_aspect_ratio(true),
                        );
                    }
                    ui.add_space(8.0);

                    ui.label(egui::RichText::new("DisplayWarp").size(20.0).strong());
                    ui.label(
                        egui::RichText::new(format!("Version: v{}", env!("CARGO_PKG_VERSION")))
                            .color(egui::Color32::GRAY),
                    );
                    ui.add_space(12.0);
                    ui.label("A tool for easily moving running applications exactly between virtual/real displays.");
                    ui.add_space(16.0);

                    ui.separator();
                    ui.add_space(8.0);
                    ui.label(egui::RichText::new("Latest Release Changelog").strong());
                    ui.add_space(4.0);

                    egui::ScrollArea::vertical()
                        .id_salt("changelog_scroll")
                        .auto_shrink([false; 2])
                        .show(ui, |ui| {
                            egui_commonmark::CommonMarkViewer::new().show(
                                ui,
                                &mut app.markdown_cache,
                                include_str!("../../CHANGELOG.md"),
                            );
                        });
                });
        });
    });
}
