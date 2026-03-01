pub mod panels;

use eframe::egui;
use egui_phosphor::regular;

use crate::app::{AppTab, WindowManagerApp};

// ─── eframe App impl ─────────────────────────────────────────────────────────

impl eframe::App for WindowManagerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint_after(std::time::Duration::from_millis(500));

        // ── First Frame Hidden Override ────────────────────────────────────
        if !self.first_frame_hidden {
            self.first_frame_hidden = true;
            if self.data.lock().start_minimized {
                hide_native_window(ctx);
            }
        }

        // ── Intercept close ────────────────────────────────────────────────
        let close_requested = ctx.input(|i| i.viewport().close_requested());
        if close_requested {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            
            if self.data.lock().close_to_tray {
                hide_native_window(ctx);
            } else {
                self.watcher_running.store(false, std::sync::atomic::Ordering::Relaxed);
                std::process::exit(0);
            }
        }

        // ── Apply theme styling ─────────────────────────────────────
        let mut style = (*ctx.style()).clone();
        style.spacing.item_spacing = egui::vec2(8.0, 6.0);
        style.spacing.button_padding = egui::vec2(10.0, 5.0);
        style.spacing.combo_height = 300.0;
        style.visuals.widgets.noninteractive.corner_radius = egui::CornerRadius::same(6);
        style.visuals.widgets.inactive.corner_radius = egui::CornerRadius::same(6);
        style.visuals.widgets.hovered.corner_radius = egui::CornerRadius::same(6);
        style.visuals.widgets.active.corner_radius = egui::CornerRadius::same(6);

        if self.dark_mode {
            style.visuals = egui::Visuals::dark();
            style.visuals.window_fill = egui::Color32::from_rgb(14, 14, 14);
            style.visuals.panel_fill = egui::Color32::from_rgb(14, 14, 14);
            style.visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(7, 7, 7);
        } else {
            style.visuals = egui::Visuals::light();
            style.visuals.window_fill = egui::Color32::WHITE;
            style.visuals.panel_fill = egui::Color32::WHITE;

            // Text: Black
            style.visuals.override_text_color = Some(egui::Color32::BLACK);
            style.visuals.widgets.noninteractive.fg_stroke.color = egui::Color32::BLACK;
            style.visuals.widgets.inactive.fg_stroke.color = egui::Color32::BLACK;
            style.visuals.widgets.hovered.fg_stroke.color = egui::Color32::BLACK;
            style.visuals.widgets.active.fg_stroke.color = egui::Color32::BLACK;

            // Button bg: CBD5E1
            let btn_bg = egui::Color32::from_rgb(203, 213, 225);
            style.visuals.widgets.inactive.bg_fill = btn_bg;
            style.visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(180, 195, 215); // Slightly darker for hover
            style.visuals.widgets.active.bg_fill = egui::Color32::from_rgb(160, 175, 195);
        }

        ctx.set_style(style);

        // ── Bottom Bar: Version + Theme Toggle ─────────────────────────────
        egui::TopBottomPanel::bottom("bottom_bar")
            .resizable(false)
            .show(ctx, |ui| {
                egui::Frame::NONE
                    .inner_margin(egui::Margin::same(8))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(format!("v{}", env!("CARGO_PKG_VERSION"))).color(
                                if self.dark_mode {
                                    egui::Color32::GRAY
                                } else {
                                    egui::Color32::from_gray(100)
                                },
                            ));

                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    let theme_icon = if self.dark_mode {
                                        regular::SUN
                                    } else {
                                        regular::MOON
                                    };
                                    if ui.button(theme_icon).clicked() {
                                        self.dark_mode = !self.dark_mode;
                                    }
                                },
                            );
                        });
                    });
            });

        // ── Central: everything else ─────────────────────────────────────
        egui::CentralPanel::default().show(ctx, |ui| {
            // Header: Logo
            if let Some(tex) = &self.logo_texture {
                ui.image(egui::load::SizedTexture::new(
                    tex.id(),
                    egui::vec2(195.0, 30.0),
                ));
            }
            ui.add_space(2.0);

            // Tab Bar
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.current_tab, AppTab::Warp, "Warp");
                ui.selectable_value(&mut self.current_tab, AppTab::Display, "Display");
                ui.selectable_value(&mut self.current_tab, AppTab::Log, "Log");
                ui.selectable_value(&mut self.current_tab, AppTab::Settings, "Settings");
            });
            ui.add_space(2.0);
            ui.separator();
            ui.add_space(8.0);

            // Tab Content
            match self.current_tab {
                AppTab::Warp => {
                    egui::ScrollArea::vertical()
                        .auto_shrink([false; 2])
                        .id_salt("warp_scroll")
                        .show(ui, |ui| {
                            // Monitor layout preview
                            let preview_idx = if self.editing_profile_idx.is_some() {
                                self.edit_profile_mon_idx
                            } else {
                                self.selected_mon_idx
                            };
                            draw_monitor_preview(self, ui, Some(preview_idx));

                            ui.add_space(8.0);

                            // 3-Column Grid: Live Mover | New Profile | Saved Profiles
                            ui.columns(3, |cols| {
                                let col_height = 480.0;

                                // Col 1: Move Live Window
                                cols[0].vertical(|ui| {
                                    ui.set_min_height(col_height);
                                    egui::Frame::group(ui.style())
                                        .inner_margin(egui::Margin::same(12))
                                        .corner_radius(egui::CornerRadius::same(8))
                                        .fill(if self.dark_mode {
                                            egui::Color32::from_rgb(25, 25, 25)
                                        } else {
                                            egui::Color32::from_rgb(248, 250, 252)
                                        })
                                        .stroke(egui::Stroke::new(
                                            1.0,
                                            if self.dark_mode {
                                                egui::Color32::from_rgb(44, 44, 44)
                                            } else {
                                                egui::Color32::from_rgb(226, 232, 240)
                                            },
                                        ))
                                        .show(ui, |ui| {
                                            ui.set_width(ui.available_width());
                                            ui.set_min_height(ui.available_height());
                                            panels::draw_live_process_mover(self, ui);
                                        });
                                });

                                // Col 2: New Profile
                                cols[1].vertical(|ui| {
                                    ui.set_min_height(col_height);
                                    egui::Frame::group(ui.style())
                                        .inner_margin(egui::Margin::same(12))
                                        .corner_radius(egui::CornerRadius::same(8))
                                        .fill(if self.dark_mode {
                                            egui::Color32::from_rgb(25, 25, 25)
                                        } else {
                                            egui::Color32::from_rgb(248, 250, 252)
                                        })
                                        .stroke(egui::Stroke::new(
                                            1.0,
                                            if self.dark_mode {
                                                egui::Color32::from_rgb(44, 44, 44)
                                            } else {
                                                egui::Color32::from_rgb(226, 232, 240)
                                            },
                                        ))
                                        .show(ui, |ui| {
                                            ui.set_width(ui.available_width());
                                            ui.set_min_height(ui.available_height());
                                            ui.label(
                                                egui::RichText::new(format!(
                                                    "{} New Profile",
                                                    regular::PLUS_CIRCLE
                                                ))
                                                .size(14.0)
                                                .strong(),
                                            );
                                            ui.add_space(8.0);
                                            panels::draw_new_profile_form(self, ui);
                                        });
                                });

                                // Col 3: Saved Profiles
                                cols[2].vertical(|ui| {
                                    ui.set_min_height(col_height);
                                    egui::Frame::group(ui.style())
                                        .inner_margin(egui::Margin::same(12))
                                        .corner_radius(egui::CornerRadius::same(8))
                                        .fill(if self.dark_mode {
                                            egui::Color32::from_rgb(25, 25, 25)
                                        } else {
                                            egui::Color32::from_rgb(248, 250, 252)
                                        })
                                        .stroke(egui::Stroke::new(
                                            1.0,
                                            if self.dark_mode {
                                                egui::Color32::from_rgb(44, 44, 44)
                                            } else {
                                                egui::Color32::from_rgb(226, 232, 240)
                                            },
                                        ))
                                        .show(ui, |ui| {
                                            ui.set_width(ui.available_width());
                                            ui.set_min_height(ui.available_height());
                                            ui.label(
                                                egui::RichText::new(format!(
                                                    "{} Saved Profiles",
                                                    regular::BOOKMARK_SIMPLE
                                                ))
                                                .size(14.0)
                                                .strong(),
                                            );
                                            ui.add_space(8.0);
                                            egui::ScrollArea::vertical()
                                                .auto_shrink([false; 2])
                                                .id_salt("saved_profiles_scroll")
                                                .show(ui, |ui| {
                                                    panels::draw_profiles_list(self, ui);
                                                });
                                        });
                                });
                            });
                        });
                }
                AppTab::Display => {
                    ui.vertical_centered(|ui| {
                        ui.add_space(100.0);
                        ui.label(
                            egui::RichText::new("Coming Soon")
                                .size(24.0)
                                .color(egui::Color32::GRAY),
                        );
                    });
                }
                AppTab::Log => {
                    egui::ScrollArea::vertical()
                        .auto_shrink([false; 2])
                        .id_salt("log_scroll")
                        .show(ui, |ui| {
                            egui::Frame::group(ui.style())
                                .inner_margin(egui::Margin::same(12))
                                .corner_radius(egui::CornerRadius::same(8))
                                .fill(if self.dark_mode {
                                    egui::Color32::from_rgb(25, 25, 25)
                                } else {
                                    egui::Color32::from_rgb(248, 250, 252)
                                })
                                .stroke(egui::Stroke::new(
                                    1.0,
                                    if self.dark_mode {
                                        egui::Color32::from_rgb(44, 44, 44)
                                    } else {
                                        egui::Color32::from_rgb(226, 232, 240)
                                    },
                                ))
                                .show(ui, |ui| {
                                    ui.set_width(ui.available_width());
                                    ui.set_min_height(ui.available_height());
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "{} Application Log",
                                            regular::TERMINAL_WINDOW
                                        ))
                                        .size(16.0)
                                        .strong(),
                                    );
                                    ui.add_space(8.0);
                                    panels::draw_status_bar(self, ui);
                                });
                        });
                }
                AppTab::Settings => {
                    egui::ScrollArea::vertical()
                        .auto_shrink([false; 2])
                        .id_salt("settings_scroll")
                        .show(ui, |ui| {
                            ui.columns(2, |cols| {
                                let col_height = 400.0;
                                // Col 1: Settings
                                cols[0].vertical(|ui| {
                                    ui.set_min_height(col_height);
                                    egui::Frame::group(ui.style())
                                        .inner_margin(egui::Margin::same(12))
                                        .corner_radius(egui::CornerRadius::same(8))
                                        .fill(if self.dark_mode {
                                            egui::Color32::from_rgb(25, 25, 25)
                                        } else {
                                            egui::Color32::from_rgb(248, 250, 252)
                                        })
                                        .stroke(egui::Stroke::new(
                                            1.0,
                                            if self.dark_mode {
                                                egui::Color32::from_rgb(44, 44, 44)
                                            } else {
                                                egui::Color32::from_rgb(226, 232, 240)
                                            },
                                        ))
                                        .show(ui, |ui| {
                                            ui.set_width(ui.available_width());
                                            ui.set_min_height(ui.available_height());
                                            ui.label(
                                                egui::RichText::new(format!(
                                                    "{} Settings",
                                                    regular::GEAR
                                                ))
                                                .size(16.0)
                                                .strong(),
                                            );
                                            ui.add_space(16.0);

                                            ui.horizontal(|ui| {
                                                ui.label("Theme:");
                                                let theme_icon = if self.dark_mode {
                                                    regular::SUN
                                                } else {
                                                    regular::MOON
                                                };
                                                if ui.button(format!("{} Toggle Theme", theme_icon)).clicked() {
                                                    self.dark_mode = !self.dark_mode;
                                                }
                                            });

                                            ui.add_space(8.0);
                                            ui.separator();
                                            ui.add_space(8.0);
                                            ui.label(egui::RichText::new("Application Behavior").strong());
                                            ui.add_space(4.0);

                                            let mut data = self.data.lock();
                                            let mut dirty = false;

                                            if ui.checkbox(&mut data.close_to_tray, "Minimize to tray on close").clicked() {
                                                dirty = true;
                                            }

                                            if ui.checkbox(&mut data.start_minimized, "Start minimized to tray").clicked() {
                                                dirty = true;
                                            }

                                            if ui.checkbox(&mut data.start_on_boot, "Start on system startup").clicked() {
                                                dirty = true;
                                                let al = WindowManagerApp::get_auto_launch();
                                                if data.start_on_boot {
                                                    let _ = al.enable();
                                                } else {
                                                    let _ = al.disable();
                                                }
                                            }

                                            ui.add_space(8.0);
                                            ui.label("Background Watcher Interval:");
                                            
                                            let mut interval = data.watcher_interval_secs;
                                            if ui.add(egui::Slider::new(&mut interval, 1..=30).suffix(" seconds")).changed() {
                                                data.watcher_interval_secs = interval;
                                                dirty = true;
                                            }
                                            
                                            ui.label(
                                                egui::RichText::new("Controls how often DisplayWarp scans application placement to enforce persistence rules. Lower speeds save CPU.")
                                                    .size(11.0)
                                                    .color(egui::Color32::from_gray(140))
                                            );

                                            if dirty {
                                                drop(data);
                                                self.save_data();
                                            }
                                        });
                                });

                                // Col 2: About
                                cols[1].vertical(|ui| {
                                    ui.set_min_height(col_height);
                                    egui::Frame::group(ui.style())
                                        .inner_margin(egui::Margin::same(12))
                                        .corner_radius(egui::CornerRadius::same(8))
                                        .fill(if self.dark_mode {
                                            egui::Color32::from_rgb(25, 25, 25)
                                        } else {
                                            egui::Color32::from_rgb(248, 250, 252)
                                        })
                                        .stroke(egui::Stroke::new(
                                            1.0,
                                            if self.dark_mode {
                                                egui::Color32::from_rgb(44, 44, 44)
                                            } else {
                                                egui::Color32::from_rgb(226, 232, 240)
                                            },
                                        ))
                                        .show(ui, |ui| {
                                            ui.set_width(ui.available_width());
                                            ui.set_min_height(ui.available_height());
                                            
                                            // Logo if available
                                            if let Some(tex) = &self.logo_texture {
                                                ui.image(egui::load::SizedTexture::new(
                                                    tex.id(),
                                                    egui::vec2(195.0, 30.0),
                                                ));
                                                ui.add_space(8.0);
                                            }

                                            ui.label(
                                                egui::RichText::new(format!(
                                                    "{} About DisplayWarp",
                                                    regular::INFO
                                                ))
                                                .size(16.0)
                                                .strong(),
                                            );
                                            ui.add_space(8.0);

                                            if self.dark_mode {
                                                ui.add(
                                                    egui::Image::new(egui::include_image!("../../assets/DisplayWarpLogoWhite.png"))
                                                        .max_width(200.0)
                                                        .maintain_aspect_ratio(true)
                                                );
                                            } else {
                                                ui.add(
                                                    egui::Image::new(egui::include_image!("../../assets/DisplayWarpLogoBlack.png"))
                                                        .max_width(200.0)
                                                        .maintain_aspect_ratio(true)
                                                );
                                            }
                                            ui.add_space(8.0);

                                            ui.label(egui::RichText::new("DisplayWarp").size(20.0).strong());
                                            ui.label(egui::RichText::new(format!("Version: v{}", env!("CARGO_PKG_VERSION"))).color(egui::Color32::GRAY));
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
                                                    egui_commonmark::CommonMarkViewer::new()
                                                        .show(ui, &mut self.markdown_cache, include_str!("../../CHANGELOG.md"));
                                                });
                                        });
                                });
                            });
                        });
                }
            }
        });
    }
}

// ─── Monitor Preview ─────────────────────────────────────────────────────────

pub fn draw_monitor_preview(
    app: &mut WindowManagerApp,
    ui: &mut egui::Ui,
    highlight_idx: Option<usize>,
) {
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
            let (rect, _) = ui.allocate_at_least(
                egui::vec2(ui.available_width(), 220.0),
                egui::Sense::hover(),
            );
            let painter = ui.painter_at(rect);

            // Draw background grid with fadeout at edges
            let step = 20.0;
            let fade_margin = 60.0;
            let max_alpha = 18.0;
            
            // Vertical lines
            let mut x = rect.left() - (rect.left() % step);
            while x < rect.right() {
                if x >= rect.left() {
                    let fade_x = ((x - rect.left()).min(rect.right() - x) / fade_margin).clamp(0.0, 1.0);
                    if fade_x > 0.0 {
                        let mut y = rect.top();
                        while y < rect.bottom() {
                            let next_y = (y + 10.0).min(rect.bottom());
                            let fade_y1 = ((y - rect.top()).min(rect.bottom() - y) / fade_margin).clamp(0.0, 1.0);
                            let fade_y2 = ((next_y - rect.top()).min(rect.bottom() - next_y) / fade_margin).clamp(0.0, 1.0);
                            let alpha = (fade_x * (fade_y1 + fade_y2) * 0.5 * max_alpha) as u8;
                            if alpha > 0 {
                                let color = if app.dark_mode {
                                    egui::Color32::from_white_alpha(alpha)
                                } else {
                                    egui::Color32::from_black_alpha(alpha)
                                };
                                painter.line_segment([egui::pos2(x, y), egui::pos2(x, next_y)], egui::Stroke::new(1.0, color));
                            }
                            y = next_y;
                        }
                    }
                }
                x += step;
            }
            
            // Horizontal lines
            let mut y = rect.top() - (rect.top() % step);
            while y < rect.bottom() {
                if y >= rect.top() {
                    let fade_y = ((y - rect.top()).min(rect.bottom() - y) / fade_margin).clamp(0.0, 1.0);
                    if fade_y > 0.0 {
                        let mut x = rect.left();
                        while x < rect.right() {
                            let next_x = (x + 10.0).min(rect.right());
                            let fade_x1 = ((x - rect.left()).min(rect.right() - x) / fade_margin).clamp(0.0, 1.0);
                            let fade_x2 = ((next_x - rect.left()).min(rect.right() - next_x) / fade_margin).clamp(0.0, 1.0);
                            let alpha = (fade_y * (fade_x1 + fade_x2) * 0.5 * max_alpha) as u8;
                            if alpha > 0 {
                                let color = if app.dark_mode {
                                    egui::Color32::from_white_alpha(alpha)
                                } else {
                                    egui::Color32::from_black_alpha(alpha)
                                };
                                painter.line_segment([egui::pos2(x, y), egui::pos2(next_x, y)], egui::Stroke::new(1.0, color));
                            }
                            x = next_x;
                        }
                    }
                }
                y += step;
            }

            if app.monitors.is_empty() {
                painter.text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "No monitors detected",
                    egui::FontId::proportional(14.0),
                    egui::Color32::GRAY,
                );
            } else {
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

                for (i, m) in app.monitors.iter().enumerate() {
                    let is_selected = highlight_idx == Some(i);
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

                    let fill = if is_selected {
                        if app.dark_mode {
                            egui::Color32::from_rgb(20, 83, 45) // Dark Green
                        } else {
                            egui::Color32::from_rgb(34, 197, 94) // #22C55E
                        }
                    } else if is_primary {
                        if app.dark_mode {
                            egui::Color32::from_rgb(76, 29, 149) // Dark Purple
                        } else {
                            egui::Color32::from_rgb(139, 92, 246) // #8B5CF6
                        }
                    } else {
                        if app.dark_mode {
                            egui::Color32::from_rgb(30, 41, 59) // Dark Slate
                        } else {
                            egui::Color32::from_rgb(148, 163, 184) // #94A3B8
                        }
                    };

                    painter.rect_filled(m_rect, 4.0, fill);
                    painter.rect_stroke(
                        m_rect,
                        egui::CornerRadius::same(4),
                        egui::Stroke::new(
                            if is_selected { 2.5 } else { 1.5 },
                            if is_selected {
                                egui::Color32::from_rgb(34, 197, 94)
                            } else {
                                egui::Color32::from_white_alpha(40)
                            },
                        ),
                        egui::StrokeKind::Outside,
                    );

                    let w = m.rect.right - m.rect.left;
                    let h = m.rect.bottom - m.rect.top;

                    // Top Left: Coordinates
                    painter.text(
                        m_rect.min + egui::vec2(6.0, 6.0),
                        egui::Align2::LEFT_TOP,
                        format!("{}, {}", m.rect.left, m.rect.top),
                        egui::FontId::proportional(10.0),
                        if app.dark_mode {
                            egui::Color32::from_white_alpha(140)
                        } else {
                            egui::Color32::from_black_alpha(140)
                        },
                    );

                    // Center: "Resolution" label
                    painter.text(
                        m_rect.center() + egui::vec2(0.0, -8.0),
                        egui::Align2::CENTER_CENTER,
                        "Resolution",
                        egui::FontId::proportional(10.0),
                        if app.dark_mode {
                            egui::Color32::from_white_alpha(180)
                        } else {
                            egui::Color32::from_black_alpha(180)
                        },
                    );

                    // Below Center: "Width x Height"
                    painter.text(
                        m_rect.center() + egui::vec2(0.0, 6.0),
                        egui::Align2::CENTER_CENTER,
                        format!("{}×{}", w, h),
                        egui::FontId::proportional(11.0),
                        if app.dark_mode {
                            egui::Color32::WHITE
                        } else {
                            egui::Color32::BLACK
                        },
                    );

                    // Bottom Right: Monitor Index
                    painter.text(
                        m_rect.right_bottom() - egui::vec2(12.0, 10.0),
                        egui::Align2::CENTER_CENTER,
                        format!("{}", i + 1),
                        egui::FontId::proportional(13.0),
                        if app.dark_mode {
                            egui::Color32::WHITE
                        } else {
                            egui::Color32::BLACK
                        },
                    );
                }
            }

            // Legend row
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(egui::RichText::new("Primary Monitor").size(12.0));
                    let (dot_rect2, _) =
                        ui.allocate_exact_size(egui::vec2(10.0, 10.0), egui::Sense::hover());
                    ui.painter().circle_filled(
                        dot_rect2.center(),
                        5.0,
                        if app.dark_mode {
                            egui::Color32::from_rgb(76, 29, 149)
                        } else {
                            egui::Color32::from_rgb(139, 92, 246)
                        },
                    );

                    ui.add_space(8.0);

                    ui.label(egui::RichText::new("Selected Monitor").size(12.0));
                    let (dot_rect, _) =
                        ui.allocate_exact_size(egui::vec2(10.0, 10.0), egui::Sense::hover());
                    ui.painter().circle_filled(
                        dot_rect.center(),
                        5.0,
                        if app.dark_mode {
                            egui::Color32::from_rgb(20, 83, 45)
                        } else {
                            egui::Color32::from_rgb(34, 197, 94)
                        },
                    );
                });
            });

            // Monitor selector buttons
            ui.horizontal(|ui| {
                for (i, m) in app.monitors.iter().enumerate() {
                    let w = m.rect.right - m.rect.left;
                    let h = m.rect.bottom - m.rect.top;
                    let label = format!("Monitor {} ({}×{})", i + 1, w, h);
                    let is_selected = i == app.selected_mon_idx;

                    let btn = if is_selected {
                        egui::Button::new(egui::RichText::new(&label).color(if app.dark_mode {
                            egui::Color32::WHITE
                        } else {
                            egui::Color32::BLACK
                        }))
                        .fill(if app.dark_mode {
                            egui::Color32::from_rgb(20, 83, 45)
                        } else {
                            egui::Color32::from_rgb(34, 197, 94)
                        })
                    } else {
                        egui::Button::new(&label)
                    };

                    if ui.add(btn).clicked() {
                        app.selected_mon_idx = i;
                    }
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .button(format!("{} Refresh Monitor", regular::ARROW_CLOCKWISE))
                        .clicked()
                    {
                        app.refresh_monitors();
                    }
                });
            });
        });
}

// ─── Native window hide (for minimize-to-tray) ──────────────────────────────

fn hide_native_window(_ctx: &egui::Context) {
    if let Some(hwnd) = get_eframe_hwnd() {
        unsafe {
            use windows::Win32::UI::WindowsAndMessaging::{
                GWL_EXSTYLE, GetWindowLongW, SW_HIDE, SW_SHOWMINNOACTIVE, SetWindowLongW,
                ShowWindow, WS_EX_TOOLWINDOW,
            };
            let _ = ShowWindow(hwnd, SW_HIDE);
            let ex = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;
            SetWindowLongW(hwnd, GWL_EXSTYLE, (ex | WS_EX_TOOLWINDOW.0) as i32);
            let _ = ShowWindow(hwnd, SW_SHOWMINNOACTIVE);
        }
    }
}

fn get_eframe_hwnd() -> Option<windows::Win32::Foundation::HWND> {
    unsafe {
        use windows::Win32::UI::WindowsAndMessaging::FindWindowW;
        use windows::core::w;
        match FindWindowW(None, w!("DisplayWarp")) {
            Ok(hwnd) if !hwnd.0.is_null() => Some(hwnd),
            _ => None,
        }
    }
}
