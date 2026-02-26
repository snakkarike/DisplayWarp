pub mod panels;

use eframe::egui;

use crate::app::WindowManagerApp;

// ─── eframe App impl ─────────────────────────────────────────────────────────

impl eframe::App for WindowManagerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint_after(std::time::Duration::from_millis(500));

        // ── Intercept close → hide to tray ──────────────────────────────
        let close_requested = ctx.input(|i| i.viewport().close_requested());
        if close_requested {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            hide_native_window(ctx);
        }

        // ── Apply dark theme styling ─────────────────────────────────────
        let mut style = (*ctx.style()).clone();
        style.spacing.item_spacing = egui::vec2(8.0, 6.0);
        style.visuals.widgets.noninteractive.rounding = egui::Rounding::same(6.0);
        style.visuals.widgets.inactive.rounding = egui::Rounding::same(6.0);
        style.visuals.widgets.hovered.rounding = egui::Rounding::same(6.0);
        style.visuals.widgets.active.rounding = egui::Rounding::same(6.0);
        ctx.set_style(style);

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.add_space(4.0);
                ui.heading(egui::RichText::new("DisplayWarp").size(20.0).strong());
                ui.add_space(8.0);

                // ── Monitor layout preview ──────────────────────────────
                let preview_idx = if self.editing_profile_idx.is_some() {
                    self.edit_profile_mon_idx
                } else {
                    self.selected_mon_idx
                };
                draw_monitor_preview(self, ui, preview_idx);

                ui.add_space(10.0);

                // ── Two-column: New Profiles | Saved Profiles ───────────
                ui.columns(2, |cols| {
                    // Left column: New Profile form
                    cols[0].group(|ui| {
                        ui.label(egui::RichText::new("🆕 New Profiles").size(14.0).strong());
                        ui.add_space(4.0);
                        panels::draw_new_profile_form(self, ui);
                    });

                    // Right column: Saved Profiles
                    cols[1].group(|ui| {
                        ui.label(egui::RichText::new("💾 Saved Profiles").size(14.0).strong());
                        ui.add_space(4.0);
                        egui::ScrollArea::vertical()
                            .max_height(250.0)
                            .id_salt("saved_profiles_scroll")
                            .show(ui, |ui| {
                                panels::draw_profiles_list(self, ui);
                            });
                    });
                });

                ui.add_space(10.0);

                // ── Move Live Window ────────────────────────────────────
                ui.group(|ui| {
                    panels::draw_live_process_mover(self, ui);
                });

                ui.add_space(10.0);

                // ── Log ─────────────────────────────────────────────────
                ui.group(|ui| {
                    ui.label(egui::RichText::new("📋 Log").size(14.0).strong());
                    panels::draw_status_bar(self, ui);
                });
            });
        });
    }
}

// ─── Monitor Preview ─────────────────────────────────────────────────────────

fn draw_monitor_preview(app: &mut WindowManagerApp, ui: &mut egui::Ui, selected_idx: usize) {
    ui.group(|ui| {
        // Preview area
        let (rect, _) = ui.allocate_at_least(
            egui::vec2(ui.available_width(), 160.0),
            egui::Sense::hover(),
        );
        let painter = ui.painter_at(rect);

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
                let is_selected = i == selected_idx;
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

                // Colors matching the mockup
                let fill = if is_selected {
                    egui::Color32::from_rgb(50, 200, 100) // green for selected
                } else if is_primary {
                    egui::Color32::from_rgb(0, 100, 200) // blue for primary
                } else {
                    egui::Color32::from_rgb(90, 50, 140) // purple for others
                };

                painter.rect_filled(m_rect, 4.0, fill);
                painter.rect_stroke(
                    m_rect,
                    4.0,
                    egui::Stroke::new(
                        if is_selected { 2.5 } else { 1.5 },
                        if is_selected {
                            egui::Color32::from_rgb(100, 255, 150)
                        } else {
                            egui::Color32::from_white_alpha(80)
                        },
                    ),
                );

                // "Resolution" label + resolution
                let w = m.rect.right - m.rect.left;
                let h = m.rect.bottom - m.rect.top;
                painter.text(
                    m_rect.center() + egui::vec2(0.0, -8.0),
                    egui::Align2::CENTER_CENTER,
                    "Resolution",
                    egui::FontId::proportional(10.0),
                    egui::Color32::from_white_alpha(180),
                );
                painter.text(
                    m_rect.center() + egui::vec2(0.0, 6.0),
                    egui::Align2::CENTER_CENTER,
                    format!("{}×{}", w, h),
                    egui::FontId::proportional(11.0),
                    egui::Color32::WHITE,
                );
                // Monitor number at bottom-right corner
                painter.text(
                    egui::pos2(m_rect.right() - 12.0, m_rect.bottom() - 10.0),
                    egui::Align2::CENTER_CENTER,
                    format!("{}", i + 1),
                    egui::FontId::proportional(13.0).into(),
                    egui::Color32::WHITE,
                );
            }
        }

        // Legend row
        ui.horizontal(|ui| {
            ui.add_space(ui.available_width() - 260.0);
            let (dot_rect, _) = ui.allocate_exact_size(egui::vec2(8.0, 8.0), egui::Sense::hover());
            ui.painter().circle_filled(
                dot_rect.center(),
                4.0,
                egui::Color32::from_rgb(50, 200, 100),
            );
            ui.label(egui::RichText::new("Selected Monitor").small());

            let (dot_rect2, _) = ui.allocate_exact_size(egui::vec2(8.0, 8.0), egui::Sense::hover());
            ui.painter().circle_filled(
                dot_rect2.center(),
                4.0,
                egui::Color32::from_rgb(0, 100, 200),
            );
            ui.label(egui::RichText::new("Primary Monitor").small());
        });

        // Monitor selector buttons
        ui.horizontal(|ui| {
            for (i, m) in app.monitors.iter().enumerate() {
                let w = m.rect.right - m.rect.left;
                let h = m.rect.bottom - m.rect.top;
                let label = format!("Monitor {} ({}×{})", i + 1, w, h);
                let is_selected = i == app.selected_mon_idx;

                let btn = if is_selected {
                    egui::Button::new(egui::RichText::new(&label).color(egui::Color32::WHITE))
                        .fill(egui::Color32::from_rgb(50, 200, 100))
                } else {
                    egui::Button::new(&label)
                };

                if ui.add(btn).clicked() {
                    app.selected_mon_idx = i;
                }
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("🔄 Refresh Monitor").clicked() {
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
        match FindWindowW(None, w!("Display Warp")) {
            Ok(hwnd) if !hwnd.0.is_null() => Some(hwnd),
            _ => None,
        }
    }
}
