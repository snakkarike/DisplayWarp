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
                    ui.add_space(12.0);

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
