use eframe::egui;
use egui_phosphor::regular;

use crate::app::WindowManagerApp;
use crate::ui::helpers::truncate_text;
use windows::Win32::Foundation::RECT;

pub fn draw_display_tab(app: &mut WindowManagerApp, ui: &mut egui::Ui, available_h: f32) {
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

    let total_width = ui.available_width();
    let col_height = available_h - 8.0;

    ui.horizontal(|ui| {
        // Col 1: Interactive Canvas & Save Form
        ui.vertical(|ui| {
            ui.set_width(total_width * 0.60 - 4.0);
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

                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.label(
                                egui::RichText::new(format!("{} Arrange Monitors", regular::CROP))
                                    .size(16.0)
                                    .strong(),
                            );
                            ui.label(
                                egui::RichText::new(
                                    "Drag and drop monitors to match your physical layout.",
                                )
                                .small()
                                .color(if app.dark_mode {
                                    egui::Color32::from_gray(140)
                                } else {
                                    egui::Color32::from_gray(100)
                                }),
                            );
                        });

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                            if ui
                                .button(format!("{} Refresh", regular::ARROWS_CLOCKWISE))
                                .on_hover_text("Redetect monitors and their current layout")
                                .clicked()
                            {
                                app.refresh_monitors();
                            }
                        });
                    });
                    ui.add_space(8.0);

                    // Interactive Dragging Canvas
                    let (response, painter) = ui.allocate_painter(
                        egui::vec2(ui.available_width(), col_height - 160.0),
                        egui::Sense::drag(),
                    );
                    let rect = response.rect;

                    painter.rect_filled(
                        rect,
                        4.0,
                        if app.dark_mode {
                            egui::Color32::from_rgb(15, 15, 15)
                        } else {
                            egui::Color32::from_rgb(255, 255, 255)
                        },
                    );

                    // Draw background grid with fadeout at edges
                    let step = 16.0;
                    let fade_margin = 60.0;
                    let max_alpha = 18.0;

                    // Vertical lines
                    let mut x = rect.left() - (rect.left() % step);
                    while x < rect.right() {
                        if x >= rect.left() {
                            let fade_x = ((x - rect.left()).min(rect.right() - x) / fade_margin)
                                .clamp(0.0, 1.0);
                            if fade_x > 0.0 {
                                let mut y = rect.top();
                                while y < rect.bottom() {
                                    let next_y = (y + 10.0).min(rect.bottom());
                                    let fade_y1 = ((y - rect.top()).min(rect.bottom() - y)
                                        / fade_margin)
                                        .clamp(0.0, 1.0);
                                    let fade_y2 = ((next_y - rect.top())
                                        .min(rect.bottom() - next_y)
                                        / fade_margin)
                                        .clamp(0.0, 1.0);
                                    let alpha =
                                        (fade_x * (fade_y1 + fade_y2) * 0.5 * max_alpha) as u8;
                                    if alpha > 0 {
                                        let color = if app.dark_mode {
                                            egui::Color32::from_white_alpha(alpha)
                                        } else {
                                            egui::Color32::from_black_alpha(alpha)
                                        };
                                        painter.line_segment(
                                            [egui::pos2(x, y), egui::pos2(x, next_y)],
                                            egui::Stroke::new(1.0, color),
                                        );
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
                            let fade_y = ((y - rect.top()).min(rect.bottom() - y) / fade_margin)
                                .clamp(0.0, 1.0);
                            if fade_y > 0.0 {
                                let mut x = rect.left();
                                while x < rect.right() {
                                    let next_x = (x + 10.0).min(rect.right());
                                    let fade_x1 = ((x - rect.left()).min(rect.right() - x)
                                        / fade_margin)
                                        .clamp(0.0, 1.0);
                                    let fade_x2 = ((next_x - rect.left())
                                        .min(rect.right() - next_x)
                                        / fade_margin)
                                        .clamp(0.0, 1.0);
                                    let alpha =
                                        (fade_y * (fade_x1 + fade_x2) * 0.5 * max_alpha) as u8;
                                    if alpha > 0 {
                                        let color = if app.dark_mode {
                                            egui::Color32::from_white_alpha(alpha)
                                        } else {
                                            egui::Color32::from_black_alpha(alpha)
                                        };
                                        painter.line_segment(
                                            [egui::pos2(x, y), egui::pos2(next_x, y)],
                                            egui::Stroke::new(1.0, color),
                                        );
                                    }
                                    x = next_x;
                                }
                            }
                        }
                        y += step;
                    }

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

                    // Group monitors by rect for rendering and interaction
                    let mut grouped_monitors: Vec<(RECT, Vec<usize>)> = Vec::new();
                    for (i, m) in app.monitors.iter().enumerate() {
                        if let Some(group) = grouped_monitors.iter_mut().find(|(r, _)| {
                            r.left == m.rect.left
                                && r.top == m.rect.top
                                && r.right == m.rect.right
                                && r.bottom == m.rect.bottom
                        }) {
                            group.1.push(i);
                        } else {
                            grouped_monitors.push((m.rect, vec![i]));
                        }
                    }

                    // Handle Dragging Logic
                    if response.drag_started() {
                        if let Some(pointer_pos) = response.interact_pointer_pos() {
                            for (rect_val, indices) in &grouped_monitors {
                                let m_rect = egui::Rect::from_min_max(
                                    center
                                        + egui::vec2(
                                            (rect_val.left - min_x) as f32 * scale
                                                - (width * scale / 2.0),
                                            (rect_val.top - min_y) as f32 * scale
                                                - (height * scale / 2.0),
                                        ),
                                    center
                                        + egui::vec2(
                                            (rect_val.right - min_x) as f32 * scale
                                                - (width * scale / 2.0),
                                            (rect_val.bottom - min_y) as f32 * scale
                                                - (height * scale / 2.0),
                                        ),
                                );
                                if m_rect.contains(pointer_pos) {
                                    // Store the first index as the primary dragging handle,
                                    // but we'll move all in the group.
                                    app.dragging_monitor_idx = Some(indices[0]);
                                    app.drag_start_pos = Some(pointer_pos);
                                    app.original_monitor_rect = Some(*rect_val);
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

                                // 40px Grid Snapping
                                let target_left =
                                    (orig_rect.left + delta_virtual_x).clamp(-16000, 16000);
                                let target_top =
                                    (orig_rect.top + delta_virtual_y).clamp(-16000, 16000);

                                let snapped_left = (target_left as f32 / 40.0).round() as i32 * 40;
                                let snapped_top = (target_top as f32 / 40.0).round() as i32 * 40;

                                let new_rect = crate::models::SerializableRect {
                                    left: snapped_left,
                                    top: snapped_top,
                                    right: snapped_left + width,
                                    bottom: snapped_top + height,
                                };

                                // Collision Detection
                                let mut collision = false;
                                for (other_idx, other_m) in app.monitors.iter().enumerate() {
                                    // Check if this other monitor is part of the same group as the dragging one
                                    let is_in_dragging_group = if let Some(grp) = grouped_monitors
                                        .iter()
                                        .find(|g: &&(RECT, Vec<usize>)| g.1.contains(&idx))
                                    {
                                        grp.1.contains(&other_idx)
                                    } else {
                                        other_idx == idx
                                    };

                                    if is_in_dragging_group {
                                        continue;
                                    }
                                    if new_rect.intersects_rect(&other_m.rect) {
                                        collision = true;
                                        break;
                                    }
                                }

                                if !collision {
                                    // Move all monitors in the group
                                    if let Some(grp) = grouped_monitors
                                        .iter()
                                        .find(|g: &&(RECT, Vec<usize>)| g.1.contains(&idx))
                                    {
                                        for &group_idx in &grp.1 {
                                            app.monitors[group_idx].rect = new_rect.to_rect();
                                        }
                                    } else {
                                        app.monitors[idx].rect = new_rect.to_rect();
                                    }
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
                    for (rect_val, indices) in &grouped_monitors {
                        let first_idx = indices[0];
                        let is_active = app.monitors[first_idx].is_active;
                        let is_primary = rect_val.left == 0 && rect_val.top == 0;
                        let is_selected = app
                            .selected_display_idx
                            .map_or(false, |si| indices.contains(&si));

                        let m_rect = egui::Rect::from_min_max(
                            center
                                + egui::vec2(
                                    (rect_val.left - min_x) as f32 * scale - (width * scale / 2.0),
                                    (rect_val.top - min_y) as f32 * scale - (height * scale / 2.0),
                                ),
                            center
                                + egui::vec2(
                                    (rect_val.right - min_x) as f32 * scale - (width * scale / 2.0),
                                    (rect_val.bottom - min_y) as f32 * scale
                                        - (height * scale / 2.0),
                                ),
                        );

                        // Interaction
                        let response =
                            ui.interact(m_rect, ui.id().with(first_idx), egui::Sense::click());
                        if response.clicked() {
                            app.selected_display_idx = Some(first_idx);
                        }

                        let fill = if is_selected {
                            if app.dark_mode {
                                egui::Color32::from_rgb(22, 163, 74)
                            } else {
                                egui::Color32::from_rgb(74, 222, 128)
                            }
                        } else if !is_active {
                            if app.dark_mode {
                                egui::Color32::from_rgb(40, 44, 52)
                            } else {
                                egui::Color32::from_rgb(220, 220, 220)
                            }
                        } else if is_primary {
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

                        if is_selected {
                            painter.rect_stroke(
                                m_rect.expand(2.0),
                                egui::CornerRadius::same(6),
                                egui::Stroke::new(
                                    2.5,
                                    if app.dark_mode {
                                        egui::Color32::WHITE
                                    } else {
                                        egui::Color32::BLACK
                                    },
                                ),
                                egui::StrokeKind::Outside,
                            );
                        } else {
                            painter.rect_stroke(
                                m_rect,
                                egui::CornerRadius::same(4),
                                egui::Stroke::new(
                                    1.5,
                                    if app.dark_mode {
                                        egui::Color32::from_white_alpha(40)
                                    } else {
                                        egui::Color32::from_black_alpha(40)
                                    },
                                ),
                                egui::StrokeKind::Middle,
                            );
                        }

                        // Label with all monitor indices in the group
                        let label = if indices.len() > 1 {
                            format!(
                                "Monitors {}",
                                indices
                                    .iter()
                                    .map(|&i| (i + 1).to_string())
                                    .collect::<Vec<_>>()
                                    .join(" | ")
                            )
                        } else {
                            format!("Monitor {}", indices[0] + 1)
                        };

                        painter.text(
                            m_rect.center(),
                            egui::Align2::CENTER_CENTER,
                            label.to_owned(),
                            egui::FontId::proportional(14.0),
                            egui::Color32::WHITE,
                        );

                        painter.text(
                            m_rect.left_top() + egui::vec2(6.0, 6.0),
                            egui::Align2::LEFT_TOP,
                            format!("{}, {}", rect_val.left, rect_val.top).to_owned(),
                            egui::FontId::proportional(10.0),
                            egui::Color32::from_white_alpha(150),
                        );
                    }

                    // Draw Inactive Displays shelf
                    let inactive_targets: Vec<_> = app
                        .display_targets
                        .iter()
                        .filter(|t| !t.is_active)
                        .cloned()
                        .collect();

                    if !inactive_targets.is_empty() {
                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new("Disconnected Displays (click to Reconnect):")
                                    .small(),
                            );
                            for target in inactive_targets {
                                let name = target
                                    .hardware_name
                                    .clone()
                                    .unwrap_or_else(|| "Unknown".to_string());
                                let btn = ui.add(
                                    egui::Button::new(format!(
                                        "{} {}",
                                        regular::PLUG,
                                        truncate_text(&name, 15)
                                    ))
                                    .small(),
                                );
                                if btn.clicked() {
                                    if let Some(tid) = target.target_id {
                                        crate::monitor::set_monitor_state(tid, "Reconnect");
                                        app.refresh_monitors();
                                    }
                                }
                            }
                        });
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
                            && !app.new_display_profile_name.is_empty()
                        {
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
                    });
                });
        });

        // Col 2: Saved Display Profiles List
        ui.vertical(|ui| {
            ui.set_width(total_width * 0.40 - 8.0);
            ui.set_min_height(col_height);

            // Monitor Settings
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
                    ui.label(
                        egui::RichText::new(format!("{} Monitor Settings", regular::GEAR))
                            .size(16.0)
                            .strong(),
                    );
                    ui.label(
                        egui::RichText::new("Click a monitor in the canvas above to configure it.")
                            .small()
                            .color(if app.dark_mode {
                                egui::Color32::from_gray(140)
                            } else {
                                egui::Color32::from_gray(100)
                            }),
                    );
                    ui.add_space(8.0);

                    if let Some(selected_idx) = app.selected_display_idx {
                        if let Some(selected_mon) = app.monitors.get(selected_idx).and_then(|m| {
                            app.display_targets
                                .iter()
                                .find(|t| t.device_name == m.device_name)
                                .cloned()
                        }) {
                            egui::Frame::group(ui.style())
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
                                    ui.set_width(ui.available_width());

                                    let mut name = selected_mon
                                        .hardware_name
                                        .clone()
                                        .unwrap_or_else(|| selected_mon.device_name.clone());
                                    if name.is_empty() {
                                        name = "Unknown Display".to_string();
                                    }

                                    ui.horizontal(|ui| {
                                        ui.label(egui::RichText::new(&name).size(14.0).strong());
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                if ui
                                                    .button(format!("{}", regular::X))
                                                    .on_hover_text("Deselect")
                                                    .clicked()
                                                {
                                                    app.selected_display_idx = None;
                                                }
                                            },
                                        );
                                    });
                                    ui.separator();

                                    let rect = app
                                        .monitors
                                        .iter()
                                        .find(|m| m.device_name == selected_mon.device_name)
                                        .map(|m| m.rect)
                                        .unwrap_or(selected_mon.rect);

                                    ui.label(format!(
                                        "Resolution: {}x{}",
                                        rect.right - rect.left,
                                        rect.bottom - rect.top
                                    ));

                                    ui.add_space(8.0);

                                    // Topology Dropdown
                                    ui.label("Topology:");
                                    let current_text = if !selected_mon.is_active {
                                        "Disconnected"
                                    } else {
                                        "Extend / Active"
                                    };

                                    let mut next_topology = None;
                                    egui::ComboBox::from_id_salt("topology_combo")
                                        .selected_text(current_text)
                                        .show_ui(ui, |ui| {
                                            if ui
                                                .selectable_label(
                                                    !selected_mon.is_active,
                                                    "Disconnected",
                                                )
                                                .clicked()
                                            {
                                                next_topology = Some("Disconnect".to_string());
                                            }
                                            if ui
                                                .selectable_label(
                                                    selected_mon.is_active,
                                                    "Extend desktop to this display",
                                                )
                                                .clicked()
                                            {
                                                next_topology = Some("Extend".to_string());
                                            }

                                            ui.separator();
                                            ui.label(
                                                egui::RichText::new("Duplicate Desktop...")
                                                    .small()
                                                    .weak(),
                                            );

                                            for (i, other) in app.display_targets.iter().enumerate()
                                            {
                                                if i != selected_idx && other.is_active {
                                                    let other_name = other
                                                        .hardware_name
                                                        .as_deref()
                                                        .unwrap_or(&other.device_name);
                                                    let my_name = selected_mon
                                                        .hardware_name
                                                        .as_deref()
                                                        .unwrap_or(&selected_mon.device_name);

                                                    // Provide duplicate option
                                                    if ui
                                                        .selectable_label(
                                                            false,
                                                            format!(
                                                                "Duplicate {} and {}",
                                                                my_name, other_name
                                                            ),
                                                        )
                                                        .clicked()
                                                    {
                                                        next_topology = Some(format!(
                                                            "DuplicateWith:{}",
                                                            other.target_id.unwrap_or(0)
                                                        ));
                                                    }
                                                }
                                            }
                                        });

                                    if let Some(next) = next_topology {
                                        if let Some(target_id) = selected_mon.target_id {
                                            crate::monitor::set_monitor_state(target_id, &next);
                                            app.refresh_monitors();
                                        }
                                    }

                                    ui.add_space(8.0);

                                    // Make Primary
                                    let is_primary = rect.left == 0 && rect.top == 0;
                                    let mut check_primary = is_primary;
                                    if ui
                                        .checkbox(&mut check_primary, "Make this my main display")
                                        .changed()
                                        && check_primary
                                    {
                                        if let Some(target_id) = selected_mon.target_id {
                                            crate::monitor::set_primary_monitor(target_id);
                                            std::thread::sleep(std::time::Duration::from_millis(
                                                500,
                                            ));
                                            app.refresh_monitors();
                                        }
                                    }
                                });
                        }
                    } else {
                        ui.vertical_centered(|ui| {
                            ui.add_space(10.0);
                            ui.label(egui::RichText::new("No monitor selected.").weak());
                            ui.add_space(10.0);
                        });
                    }
                }); // End Monitor Settings

            ui.add_space(4.0);

            // Saved Display Profiles List
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
                    ui.set_min_height(ui.available_height()); // fill the rest of the height

                    ui.label(
                        egui::RichText::new(format!("{} Saved Profiles", regular::BOOKMARK_SIMPLE))
                            .size(16.0)
                            .strong(),
                    );
                    ui.label(
                        egui::RichText::new("Quickly restore a saved multi-monitor layout.")
                            .small()
                            .color(if app.dark_mode {
                                egui::Color32::from_gray(140)
                            } else {
                                egui::Color32::from_gray(100)
                            }),
                    );
                    ui.add_space(8.0);

                    let display_profiles = app.data.lock().display_profiles.clone();

                    if display_profiles.is_empty() {
                        ui.label(
                            egui::RichText::new("No display profiles saved.")
                                .color(egui::Color32::GRAY),
                        );
                    }

                    let mut to_remove: Option<usize> = None;
                    let mut to_move_up: Option<usize> = None;
                    let mut to_move_down: Option<usize> = None;

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
                                        ui.vertical(|ui| {
                                            // Top Row: Name on left, Arrows on right
                                            ui.horizontal(|ui| {
                                                ui.label(egui::RichText::new(&p.name).strong());
                                                ui.with_layout(
                                                    egui::Layout::right_to_left(
                                                        egui::Align::Center,
                                                    ),
                                                    |ui| {
                                                        if ui.button(regular::CARET_DOWN).clicked()
                                                        {
                                                            to_move_down = Some(i);
                                                        }
                                                        if ui.button(regular::CARET_UP).clicked() {
                                                            to_move_up = Some(i);
                                                        }
                                                    },
                                                );
                                            });

                                            ui.add_space(4.0);

                                            // Bottom Row: The 3 main action buttons
                                            ui.horizontal(|ui| {
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
                                                    .button(format!("{} View", regular::EYE))
                                                    .clicked()
                                                {
                                                    for profile_mon in &p.monitors {
                                                        if let Some(mon) =
                                                            app.monitors.iter_mut().find(|m| {
                                                                m.device_name
                                                                    == profile_mon.device_name
                                                            })
                                                        {
                                                            mon.rect = profile_mon.rect.to_rect();
                                                        }
                                                    }
                                                }
                                                if ui
                                                    .button(format!("{} Delete", regular::TRASH))
                                                    .clicked()
                                                {
                                                    to_remove = Some(i);
                                                }
                                            });
                                        });
                                    });
                                ui.add_space(4.0);
                            }
                        });

                    if let Some(i) = to_move_up {
                        if i > 0 {
                            app.data.lock().display_profiles.swap(i, i - 1);
                            app.save_data();
                            let profiles = app.data.lock().profiles.clone();
                            let display_profiles = app.data.lock().display_profiles.clone();
                            if let Some(t) = &app.tray {
                                t.refresh_menu(&profiles, &display_profiles);
                            }
                        }
                    }

                    if let Some(i) = to_move_down {
                        let len = app.data.lock().display_profiles.len();
                        if i < len - 1 {
                            app.data.lock().display_profiles.swap(i, i + 1);
                            app.save_data();
                            let profiles = app.data.lock().profiles.clone();
                            let display_profiles = app.data.lock().display_profiles.clone();
                            if let Some(t) = &app.tray {
                                t.refresh_menu(&profiles, &display_profiles);
                            }
                        }
                    }

                    if let Some(idx) = to_remove {
                        let name = app.data.lock().display_profiles[idx].name.clone();
                        app.data.lock().display_profiles.remove(idx);
                        app.save_data();

                        let profiles = app.data.lock().profiles.clone();
                        let display_profiles = app.data.lock().display_profiles.clone();
                        if let Some(t) = &app.tray {
                            t.refresh_menu(&profiles, &display_profiles);
                        }

                        crate::app::WindowManagerApp::push_status(
                            &app.status_message,
                            &app.status_log,
                            format!("🗑 Display layout deleted: {}", name),
                        );
                    }
                });
        });
    });
}
