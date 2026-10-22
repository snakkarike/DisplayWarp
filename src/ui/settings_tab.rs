use eframe::egui;
use crate::ui::icons::regular;

use crate::app::WindowManagerApp;

pub fn draw_settings_tab(app: &mut WindowManagerApp, ui: &mut egui::Ui, available_h: f32) {
    ui.columns(2, |cols| {
        let col_height = (available_h - 8.0).max(650.0);
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
                    1.0_f32,
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
                            1.0_f32,
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
                                        let _exe_dir = std::env::current_exe()
                                            .unwrap_or_else(|_| std::path::PathBuf::from("."))
                                            .parent()
                                            .unwrap_or_else(|| std::path::Path::new("."))
                                            .to_path_buf();

                                        let new_config_file = folder.join("monitor_config.json");

                                        if config_path.exists() && !new_config_file.exists() {
                                            let _ = std::fs::copy(&config_path, &new_config_file);
                                        }

                                        let config_base = crate::app::WindowManagerApp::get_config_base_dir();
                                        if !config_base.exists() {
                                            let _ = std::fs::create_dir_all(&config_base);
                                        }

                                        let _ = std::fs::write(
                                            config_base.join("config_location.txt"),
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
                            1.0_f32,
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
                                            app.theme = crate::ui::theme::Theme::new(app.dark_mode);
                                            app.theme.apply_to_ui(ui);
                                            crate::tray::apply_window_dark_mode(app.dark_mode);
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
                            1.0_f32,
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

                            ui.add_space(4.0);
                            if ui
                                .checkbox(&mut data.check_updates_on_startup, "Automatically check for updates on startup")
                                .clicked()
                            {
                                dirty = true;
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
                            1.0_f32,
                            if app.dark_mode {
                                egui::Color32::from_rgb(44, 44, 44)
                            } else {
                                egui::Color32::from_rgb(226, 232, 240)
                            },
                        ))
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());

                            let mut data = app.data.lock();
                            let mut dirty = false;

                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new("Background Window Watcher").strong());
                                let is_enabled = data.watcher_enabled;
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    let badge_bg = if is_enabled {
                                        if app.dark_mode {
                                            egui::Color32::from_rgb(22, 101, 52)
                                        } else {
                                            egui::Color32::from_rgb(220, 252, 231)
                                        }
                                    } else {
                                        if app.dark_mode {
                                            egui::Color32::from_rgb(55, 65, 81)
                                        } else {
                                            egui::Color32::from_rgb(243, 244, 246)
                                        }
                                    };
                                    let badge_fg = if is_enabled {
                                        if app.dark_mode {
                                            egui::Color32::from_rgb(134, 239, 172)
                                        } else {
                                            egui::Color32::from_rgb(21, 128, 61)
                                        }
                                    } else {
                                        if app.dark_mode {
                                            egui::Color32::from_gray(180)
                                        } else {
                                            egui::Color32::from_gray(100)
                                        }
                                    };
                                    egui::Frame::new()
                                        .fill(badge_bg)
                                        .corner_radius(egui::CornerRadius::same(10))
                                        .inner_margin(egui::Margin::symmetric(8, 2))
                                        .show(ui, |ui| {
                                            ui.label(
                                                egui::RichText::new(if is_enabled { "Active" } else { "Disabled" })
                                                    .size(10.5)
                                                    .color(badge_fg),
                                            );
                                        });
                                });
                            });
                            ui.add_space(6.0);

                            if ui
                                .checkbox(&mut data.watcher_enabled, "Enable background window watcher")
                                .on_hover_text("When enabled, DisplayWarp scans in the background to ensure persistent profile windows stay on their assigned monitors.")
                                .clicked()
                            {
                                dirty = true;
                            }

                            if data.watcher_enabled {
                                ui.add_space(6.0);
                                ui.label(egui::RichText::new("Scan Interval:").small());
                                let mut interval = data.watcher_interval_secs;
                                if ui
                                    .add(egui::Slider::new(&mut interval, 1..=30).suffix(" seconds"))
                                    .changed()
                                {
                                    data.watcher_interval_secs = interval;
                                    dirty = true;
                                }

                                ui.add_space(4.0);
                                ui.label(
                                    egui::RichText::new("Controls how often DisplayWarp scans application placement to enforce persistence rules. Higher intervals save CPU.")
                                        .size(11.0)
                                        .color(egui::Color32::from_gray(140)),
                                );
                            } else {
                                ui.add_space(4.0);
                                ui.label(
                                    egui::RichText::new("Watcher is currently disabled. DisplayWarp will not scan or reposition windows in the background, minimizing CPU and battery usage.")
                                        .size(11.0)
                                        .color(egui::Color32::from_gray(140)),
                                );
                            }

                            if dirty {
                                let enabled = data.watcher_enabled;
                                let interval = data.watcher_interval_secs;
                                drop(data);
                                app.save_data();
                                crate::app::WindowManagerApp::push_status(
                                    &app.status_message,
                                    &app.status_log,
                                    if enabled {
                                        format!("⚙️ Background watcher enabled ({}s interval).", interval)
                                    } else {
                                        "⚙️ Background watcher disabled.".to_string()
                                    },
                                );
                            }
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
                            1.0_f32,
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
                    1.0_f32,
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

                    let logo_option = if app.dark_mode {
                        &app.logo_texture_white
                    } else {
                        &app.logo_texture
                    };
                    if let Some(tex) = logo_option {
                        ui.image(egui::load::SizedTexture::new(
                            tex.id(),
                            egui::vec2(192.0, 30.0),
                        ));
                    }
                    ui.add_space(6.0);

                    ui.label(
                        egui::RichText::new(format!("Version: v{}", env!("CARGO_PKG_VERSION")))
                            .small()
                            .color(if app.dark_mode {
                                egui::Color32::from_gray(160)
                            } else {
                                egui::Color32::from_gray(100)
                            }),
                    );
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new(
                            "A tool for easily moving running applications exactly between virtual/real displays.",
                        )
                        .color(if app.dark_mode {
                            egui::Color32::from_gray(200)
                        } else {
                            egui::Color32::from_gray(70)
                        }),
                    );
                    ui.add_space(10.0);

                    // ── Software Updates Card ──
                    let update_status = app.updater.status.lock().clone();
                    let is_busy = app.updater.is_busy.load(std::sync::atomic::Ordering::SeqCst);

                    let card_bg = match &update_status {
                        crate::updater::UpdateStatus::UpdateAvailable { .. } => {
                            if app.dark_mode {
                                egui::Color32::from_rgb(45, 30, 65)
                            } else {
                                egui::Color32::from_rgb(243, 232, 255)
                            }
                        }
                        crate::updater::UpdateStatus::ReadyToRestart { .. } => {
                            if app.dark_mode {
                                egui::Color32::from_rgb(20, 50, 30)
                            } else {
                                egui::Color32::from_rgb(236, 253, 245)
                            }
                        }
                        _ => {
                            if app.dark_mode {
                                egui::Color32::from_rgb(32, 32, 32)
                            } else {
                                egui::Color32::from_rgb(241, 245, 249)
                            }
                        }
                    };

                    let card_stroke = match &update_status {
                        crate::updater::UpdateStatus::UpdateAvailable { .. } => {
                            egui::Stroke::new(1.0, egui::Color32::from_rgb(147, 51, 234))
                        }
                        crate::updater::UpdateStatus::ReadyToRestart { .. } => {
                            egui::Stroke::new(1.0, egui::Color32::from_rgb(34, 197, 94))
                        }
                        _ => egui::Stroke::new(
                            1.0,
                            if app.dark_mode {
                                egui::Color32::from_rgb(48, 48, 48)
                            } else {
                                egui::Color32::from_rgb(226, 232, 240)
                            },
                        ),
                    };

                    egui::Frame::group(ui.style())
                        .inner_margin(egui::Margin::symmetric(10, 8))
                        .corner_radius(egui::CornerRadius::same(6))
                        .fill(card_bg)
                        .stroke(card_stroke)
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            match &update_status {
                                crate::updater::UpdateStatus::Idle => {
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            egui::RichText::new("Check for new DisplayWarp releases.")
                                                .small()
                                                .color(egui::Color32::GRAY),
                                        );
                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                            if ui
                                                .button(format!("{} Check for Updates", regular::ARROWS_CLOCKWISE))
                                                .clicked()
                                            {
                                                app.updater.check_for_updates();
                                            }
                                        });
                                    });
                                }
                                crate::updater::UpdateStatus::Checking => {
                                    ui.horizontal(|ui| {
                                        ui.spinner();
                                        ui.label(egui::RichText::new("Checking GitHub for updates...").small());
                                    });
                                }
                                crate::updater::UpdateStatus::UpToDate { checked_version } => {
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            egui::RichText::new(format!("{} Up to date (v{})", regular::CHECK, checked_version))
                                                .small()
                                                .color(egui::Color32::from_rgb(34, 197, 94)),
                                        );
                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                            if ui
                                                .button(format!("{} Check Again", regular::ARROWS_CLOCKWISE))
                                                .clicked()
                                            {
                                                app.updater.check_for_updates();
                                            }
                                        });
                                    });
                                }
                                crate::updater::UpdateStatus::UpdateAvailable {
                                    version,
                                    release_name: _,
                                    release_notes: _,
                                    download_url,
                                    html_url,
                                    asset_size,
                                } => {
                                    ui.vertical(|ui| {
                                        ui.horizontal(|ui| {
                                            ui.label(
                                                egui::RichText::new(format!("🎉 New Release Available: v{}", version))
                                                    .strong()
                                                    .color(if app.dark_mode {
                                                        egui::Color32::from_rgb(216, 180, 254)
                                                    } else {
                                                        egui::Color32::from_rgb(126, 34, 206)
                                                    }),
                                            );
                                            if *asset_size > 0 {
                                                let mb = *asset_size as f64 / (1024.0 * 1024.0);
                                                ui.label(
                                                    egui::RichText::new(format!("({:.1} MB)", mb))
                                                        .small()
                                                        .color(egui::Color32::GRAY),
                                                );
                                            }
                                        });

                                        ui.add_space(4.0);
                                        ui.horizontal(|ui| {
                                            let update_btn = egui::Button::new(
                                                egui::RichText::new(format!("{} Download & Install", regular::DOWNLOAD_SIMPLE))
                                                    .color(egui::Color32::WHITE),
                                            )
                                            .fill(if app.dark_mode {
                                                egui::Color32::from_rgb(109, 40, 217)
                                            } else {
                                                egui::Color32::from_rgb(124, 58, 237)
                                            });

                                            if ui.add(update_btn).clicked() && !is_busy {
                                                app.updater.start_download(version.clone(), download_url.clone());
                                            }

                                            if !html_url.is_empty() {
                                                if ui
                                                    .button(format!("{} Release Notes", regular::EXPORT))
                                                    .clicked()
                                                {
                                                    let _ = std::process::Command::new("cmd")
                                                        .args(&["/C", "start", html_url])
                                                        .spawn();
                                                }
                                            }
                                        });
                                    });
                                }
                                crate::updater::UpdateStatus::Downloading {
                                    version,
                                    downloaded_bytes,
                                    total_bytes,
                                    progress,
                                } => {
                                    ui.vertical(|ui| {
                                        ui.horizontal(|ui| {
                                            ui.spinner();
                                            let downloaded_mb = *downloaded_bytes as f64 / (1024.0 * 1024.0);
                                            let total_mb = *total_bytes as f64 / (1024.0 * 1024.0);
                                            if *total_bytes > 0 {
                                                ui.label(format!(
                                                    "Downloading v{}... ({:.1} / {:.1} MB)",
                                                    version, downloaded_mb, total_mb
                                                ));
                                            } else {
                                                ui.label(format!("Downloading v{}... ({:.1} MB)", version, downloaded_mb));
                                            }
                                        });
                                        ui.add_space(4.0);
                                        ui.add(egui::ProgressBar::new(*progress).animate(true));
                                    });
                                }
                                crate::updater::UpdateStatus::ReadyToRestart { version } => {
                                    ui.vertical(|ui| {
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "{} Version v{} downloaded and ready to install!",
                                                regular::CHECK,
                                                version
                                            ))
                                            .strong()
                                            .color(egui::Color32::from_rgb(34, 197, 94)),
                                        );
                                        ui.add_space(4.0);
                                        let restart_btn = egui::Button::new(
                                            egui::RichText::new(format!("{} Restart & Apply Update", regular::ARROWS_CLOCKWISE))
                                                .color(egui::Color32::WHITE),
                                        )
                                        .fill(egui::Color32::from_rgb(22, 101, 52));

                                        if ui.add(restart_btn).clicked() {
                                            if let Err(e) = crate::updater::UpdateManager::apply_and_restart() {
                                                *app.updater.status.lock() = crate::updater::UpdateStatus::Error(e);
                                            }
                                        }
                                    });
                                }
                                crate::updater::UpdateStatus::Error(err) => {
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            egui::RichText::new(format!("⚠️ {}", err))
                                                .small()
                                                .color(egui::Color32::from_rgb(239, 68, 68)),
                                        );
                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                            if ui.button("Retry").clicked() {
                                                app.updater.check_for_updates();
                                            }
                                        });
                                    });
                                }
                            }
                        });
                    ui.add_space(10.0);

                    ui.separator();
                    ui.add_space(8.0);
                    ui.label(
                        egui::RichText::new(format!("{} Latest Release Changelog", regular::FILE))
                            .strong(),
                    );
                    ui.add_space(6.0);

                    egui::ScrollArea::vertical()
                        .id_salt("changelog_scroll")
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            for line in include_str!("../../CHANGELOG.md").lines() {
                                render_markdown_line(ui, line, app.dark_mode);
                            }
                        });
                });
        });
    });
}

fn render_markdown_line(ui: &mut egui::Ui, line: &str, dark_mode: bool) {
    let line = line.trim_end();
    if line.starts_with("# ") {
        ui.add_space(8.0);
        ui.add(egui::Label::new(egui::RichText::new(&line[2..]).size(16.0).strong()).wrap());
        ui.add_space(2.0);
    } else if line.starts_with("## ") {
        ui.add_space(8.0);
        ui.add(egui::Label::new(egui::RichText::new(&line[3..]).size(14.0).strong()).wrap());
        ui.add_space(2.0);
    } else if line.starts_with("### ") {
        ui.add_space(6.0);
        ui.add(
            egui::Label::new(
                egui::RichText::new(&line[4..])
                    .strong()
                    .size(13.0)
                    .color(if dark_mode {
                        egui::Color32::from_rgb(147, 197, 253)
                    } else {
                        egui::Color32::from_rgb(37, 99, 235)
                    }),
            )
            .wrap(),
        );
        ui.add_space(2.0);
    } else if line.starts_with("---") {
        ui.add_space(4.0);
        ui.separator();
        ui.add_space(4.0);
    } else if let Some(bullet) = line.strip_prefix("- ") {
        let max_w = ui.available_width().max(80.0);
        let job = build_formatted_job(bullet, "• ", dark_mode, max_w, ui.style());
        ui.add(egui::Label::new(job).wrap());
    } else if let Some(sub_bullet) = line.strip_prefix("  - ") {
        let max_w = ui.available_width().max(80.0);
        let job = build_formatted_job(sub_bullet, "    ⁃ ", dark_mode, max_w, ui.style());
        ui.add(egui::Label::new(job).wrap());
    } else if !line.trim().is_empty() {
        let max_w = ui.available_width().max(80.0);
        let job = build_formatted_job(line, "", dark_mode, max_w, ui.style());
        ui.add(egui::Label::new(job).wrap());
    } else {
        ui.add_space(3.0);
    }
}

fn build_formatted_job(
    text: &str,
    prefix: &str,
    dark_mode: bool,
    max_width: f32,
    style: &egui::Style,
) -> egui::text::LayoutJob {
    use egui::text::{LayoutJob, TextFormat};

    let mut job = LayoutJob::default();
    job.wrap.max_width = max_width;
    job.wrap.max_rows = usize::MAX;
    job.wrap.break_anywhere = false;

    let base_color = if dark_mode {
        egui::Color32::from_gray(210)
    } else {
        egui::Color32::from_gray(40)
    };
    let bullet_color = if dark_mode {
        egui::Color32::from_rgb(167, 139, 250)
    } else {
        egui::Color32::from_rgb(124, 58, 237)
    };
    let code_fg = if dark_mode {
        egui::Color32::from_rgb(244, 114, 182)
    } else {
        egui::Color32::from_rgb(190, 24, 93)
    };
    let code_bg = if dark_mode {
        egui::Color32::from_rgb(45, 45, 45)
    } else {
        egui::Color32::from_rgb(226, 232, 240)
    };

    let font_normal = egui::TextStyle::Body.resolve(style);
    let mut font_bold = font_normal.clone();
    font_bold.size += 0.5;

    let font_mono = egui::TextStyle::Monospace.resolve(style);

    if !prefix.is_empty() {
        job.append(
            prefix,
            0.0,
            TextFormat {
                font_id: font_normal.clone(),
                color: bullet_color,
                ..Default::default()
            },
        );
    }

    let mut rest = text;
    while !rest.is_empty() {
        if let Some(stripped) = rest.strip_prefix("**") {
            if let Some(end_idx) = stripped.find("**") {
                let bold_text = &stripped[..end_idx];
                job.append(
                    bold_text,
                    0.0,
                    TextFormat {
                        font_id: font_bold.clone(),
                        color: if dark_mode {
                            egui::Color32::WHITE
                        } else {
                            egui::Color32::BLACK
                        },
                        ..Default::default()
                    },
                );
                rest = &stripped[end_idx + 2..];
                continue;
            }
        }
        if let Some(stripped) = rest.strip_prefix('`') {
            if let Some(end_idx) = stripped.find('`') {
                let code_text = &stripped[..end_idx];
                job.append(
                    code_text,
                    0.0,
                    TextFormat {
                        font_id: font_mono.clone(),
                        color: code_fg,
                        background: code_bg,
                        ..Default::default()
                    },
                );
                rest = &stripped[end_idx + 1..];
                continue;
            }
        }
        if let Some(stripped) = rest.strip_prefix('[') {
            if let Some(close_bracket) = stripped.find(']') {
                let link_text = &stripped[..close_bracket];
                let remainder = &stripped[close_bracket + 1..];
                if remainder.starts_with('(') {
                    if let Some(close_paren) = remainder.find(')') {
                        job.append(
                            link_text,
                            0.0,
                            TextFormat {
                                font_id: font_bold.clone(),
                                color: if dark_mode {
                                    egui::Color32::from_rgb(147, 197, 253)
                                } else {
                                    egui::Color32::from_rgb(37, 99, 235)
                                },
                                ..Default::default()
                            },
                        );
                        rest = &remainder[close_paren + 1..];
                        continue;
                    }
                }
            }
        }
        let next_bold = rest.find("**").unwrap_or(rest.len());
        let next_code = rest.find('`').unwrap_or(rest.len());
        let next_link = rest.find('[').unwrap_or(rest.len());
        let next_delim = if next_bold == 0 {
            2.min(rest.len())
        } else if next_code == 0 || next_link == 0 {
            1.min(rest.len())
        } else {
            next_bold.min(next_code).min(next_link)
        };
        let plain = &rest[..next_delim];
        if !plain.is_empty() {
            job.append(
                plain,
                0.0,
                TextFormat {
                    font_id: font_normal.clone(),
                    color: base_color,
                    ..Default::default()
                },
            );
        }
        rest = &rest[next_delim..];
    }

    job
}
