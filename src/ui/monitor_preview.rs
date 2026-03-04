use eframe::egui;
use egui_phosphor::regular;

use crate::app::WindowManagerApp;

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
                    let fade_x =
                        ((x - rect.left()).min(rect.right() - x) / fade_margin).clamp(0.0, 1.0);
                    if fade_x > 0.0 {
                        let mut y = rect.top();
                        while y < rect.bottom() {
                            let next_y = (y + 10.0).min(rect.bottom());
                            let fade_y1 = ((y - rect.top()).min(rect.bottom() - y) / fade_margin)
                                .clamp(0.0, 1.0);
                            let fade_y2 = ((next_y - rect.top()).min(rect.bottom() - next_y)
                                / fade_margin)
                                .clamp(0.0, 1.0);
                            let alpha = (fade_x * (fade_y1 + fade_y2) * 0.5 * max_alpha) as u8;
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
                    let fade_y =
                        ((y - rect.top()).min(rect.bottom() - y) / fade_margin).clamp(0.0, 1.0);
                    if fade_y > 0.0 {
                        let mut x = rect.left();
                        while x < rect.right() {
                            let next_x = (x + 10.0).min(rect.right());
                            let fade_x1 = ((x - rect.left()).min(rect.right() - x) / fade_margin)
                                .clamp(0.0, 1.0);
                            let fade_x2 = ((next_x - rect.left()).min(rect.right() - next_x)
                                / fade_margin)
                                .clamp(0.0, 1.0);
                            let alpha = (fade_y * (fade_x1 + fade_x2) * 0.5 * max_alpha) as u8;
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
