pub mod display_tab;
pub mod helpers;
pub mod log_tab;
pub mod monitor_preview;
pub mod settings_tab;
pub mod warp_tab;

use eframe::egui;
use egui_phosphor::regular;
use std::sync::Arc;
use std::sync::atomic::Ordering;

use crate::app::{AppTab, WindowManagerApp};

// ─── eframe App impl ─────────────────────────────────────────────────────────

impl eframe::App for WindowManagerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint_after(std::time::Duration::from_millis(500));

        // ── Tray popup polling ─────────────────────────────────────────────
        if self.show_tray_popup.is_none() {
            if let Some(tray) = &self.tray {
                if let Ok(req) = tray.popup_rx.try_recv() {
                    self.show_tray_popup = Some(req);
                }
            }
        }

        if let Some(req) = &self.show_tray_popup {
            let dark = req.dark;
            let pos = egui::pos2(req.x, req.y);
            let size = egui::vec2(req.w, req.h);
            let data = Arc::clone(&self.data);
            let status_message = Arc::clone(&self.status_message);
            let status_log = Arc::clone(&self.status_log);
            let watcher_running = Arc::clone(&self.watcher_running);
            let mut close_popup = false;

            ctx.show_viewport_immediate(
                egui::ViewportId::from_hash_of("tray_popup"),
                egui::ViewportBuilder::default()
                    .with_position(pos)
                    .with_inner_size(size)
                    .with_decorations(false)
                    .with_transparent(true)
                    .with_always_on_top()
                    .with_resizable(false)
                    .with_taskbar(false),
                |ctx, _class| {
                    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                        close_popup = true;
                    }
                    if ctx.input(|i| i.pointer.any_click()) {
                        if let Some(click_pos) = ctx.input(|i| i.pointer.interact_pos()) {
                            let screen = ctx.input(|i| {
                                i.viewport().inner_rect.unwrap_or(egui::Rect::EVERYTHING)
                            });
                            if !screen.contains(click_pos) {
                                close_popup = true;
                            }
                        }
                    }

                    let bg = if dark {
                        egui::Color32::from_rgb(28, 28, 30)
                    } else {
                        egui::Color32::from_rgb(245, 245, 247)
                    };
                    let text_color = if dark {
                        egui::Color32::from_rgb(235, 235, 245)
                    } else {
                        egui::Color32::from_rgb(20, 20, 25)
                    };
                    let hover_bg = if dark {
                        egui::Color32::from_rgb(50, 50, 58)
                    } else {
                        egui::Color32::from_rgb(215, 215, 225)
                    };
                    let sep_color = if dark {
                        egui::Color32::from_rgb(60, 60, 68)
                    } else {
                        egui::Color32::from_rgb(200, 200, 210)
                    };

                    let frame = egui::Frame::new()
                        .fill(bg)
                        .corner_radius(egui::CornerRadius::same(10))
                        .inner_margin(egui::Margin::symmetric(0, 6))
                        .shadow(egui::Shadow {
                            offset: [0, 4],
                            blur: 20,
                            spread: 0,
                            color: egui::Color32::from_black_alpha(80),
                        });

                    egui::CentralPanel::default().frame(frame).show(ctx, |ui| {
                        ui.set_min_width(220.0);

                        let (profiles, layouts) = {
                            let d = data.lock();
                            (d.profiles.clone(), d.display_profiles.clone())
                        };

                        if popup_item(ui, "👁", "Show", text_color, hover_bg).clicked() {
                            crate::tray::show_window_native_pub();
                            close_popup = true;
                        }

                        popup_separator(ui, sep_color);
                        popup_section(ui, "⚡  Warp Profiles", text_color);

                        if profiles.is_empty() {
                            popup_empty(ui, "(No profiles)", text_color);
                        } else {
                            for p in profiles.iter() {
                                if popup_item(ui, "⚡", &p.name, text_color, hover_bg).clicked() {
                                    let profile = p.clone();
                                    let sm = Arc::clone(&status_message);
                                    let sl = Arc::clone(&status_log);
                                    std::thread::spawn(move || {
                                        crate::app::WindowManagerApp::launch_profile(
                                            &profile, sm, sl,
                                        );
                                    });
                                    close_popup = true;
                                }
                            }
                        }

                        popup_separator(ui, sep_color);
                        popup_section(ui, "🖥  Display Profiles", text_color);

                        if layouts.is_empty() {
                            popup_empty(ui, "(No layouts)", text_color);
                        } else {
                            for l in layouts.iter() {
                                if popup_item(ui, "🖥", &l.name, text_color, hover_bg).clicked() {
                                    let layout = l.clone();
                                    let sm = Arc::clone(&status_message);
                                    let sl = Arc::clone(&status_log);
                                    std::thread::spawn(move || {
                                        crate::app::WindowManagerApp::apply_display_layout(
                                            &layout, sm, sl,
                                        );
                                    });
                                    close_popup = true;
                                }
                            }
                        }

                        popup_separator(ui, sep_color);

                        if popup_item(
                            ui,
                            "✕",
                            "Quit",
                            egui::Color32::from_rgb(255, 90, 90),
                            hover_bg,
                        )
                        .clicked()
                        {
                            watcher_running.store(false, Ordering::Relaxed);
                            std::process::exit(0);
                        }
                    });

                    if close_popup {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                },
            );

            if close_popup {
                self.show_tray_popup = None;
            }
        }

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
                self.watcher_running.store(false, Ordering::Relaxed);
                std::process::exit(0);
            }
        }

        // ── Apply theme styling ────────────────────────────────────────────
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
            style.visuals.override_text_color = Some(egui::Color32::BLACK);
            style.visuals.widgets.noninteractive.fg_stroke.color = egui::Color32::BLACK;
            style.visuals.widgets.inactive.fg_stroke.color = egui::Color32::BLACK;
            style.visuals.widgets.hovered.fg_stroke.color = egui::Color32::BLACK;
            style.visuals.widgets.active.fg_stroke.color = egui::Color32::BLACK;
            let btn_bg = egui::Color32::from_rgb(203, 213, 225);
            style.visuals.widgets.inactive.bg_fill = btn_bg;
            style.visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(180, 195, 215);
            style.visuals.widgets.active.bg_fill = egui::Color32::from_rgb(160, 175, 195);
        }
        ctx.set_style(style);

        // ── Bottom Bar ─────────────────────────────────────────────────────
        egui::TopBottomPanel::bottom("bottom_bar")
            .resizable(false)
            .show(ctx, |ui| {
                egui::Frame::NONE
                    .inner_margin(egui::Margin::same(8))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.with_layout(
                                egui::Layout::left_to_right(egui::Align::Center),
                                |ui| {
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "v{}",
                                            env!("CARGO_PKG_VERSION")
                                        ))
                                        .color(
                                            if self.dark_mode {
                                                egui::Color32::GRAY
                                            } else {
                                                egui::Color32::from_gray(100)
                                            },
                                        ),
                                    );
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
                                },
                            );
                        });
                    });
            });

        // ── Central Panel ──────────────────────────────────────────────────
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_space(2.0);

            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    let logo_option = if self.dark_mode {
                        &self.logo_texture_white
                    } else {
                        &self.logo_texture
                    };
                    if let Some(tex) = logo_option {
                        ui.image(egui::load::SizedTexture::new(
                            tex.id(),
                            egui::vec2(146.0, 22.0),
                        ));
                        ui.add_space(8.0);
                    }

                    ui.visuals_mut().selection.bg_fill = if self.dark_mode {
                        egui::Color32::from_rgb(76, 29, 149)
                    } else {
                        egui::Color32::from_rgb(139, 92, 246)
                    };
                    ui.visuals_mut().selection.stroke.color = egui::Color32::WHITE;

                    ui.selectable_value(&mut self.current_tab, AppTab::Warp, "Warp");
                    ui.selectable_value(&mut self.current_tab, AppTab::Display, "Display");
                    ui.selectable_value(&mut self.current_tab, AppTab::Log, "Log");
                    ui.selectable_value(&mut self.current_tab, AppTab::Settings, "Settings");
                });
            });
            ui.add_space(2.0);
            ui.separator();
            ui.add_space(8.0);

            match self.current_tab {
                AppTab::Warp => {
                    egui::ScrollArea::vertical()
                        .auto_shrink([false; 2])
                        .id_salt("warp_scroll")
                        .show(ui, |ui| {
                            warp_tab::draw_warp_tab(self, ui);
                        });
                }
                AppTab::Display => {
                    let available_h = ui.available_rect_before_wrap().height();
                    display_tab::draw_display_tab(self, ui, available_h);
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
                                    ui.label(
                                        egui::RichText::new(
                                            "View background service activity and errors.",
                                        )
                                        .small()
                                        .color(
                                            if self.dark_mode {
                                                egui::Color32::from_gray(140)
                                            } else {
                                                egui::Color32::from_gray(100)
                                            },
                                        ),
                                    );
                                    ui.add_space(8.0);
                                    log_tab::draw_status_bar(self, ui);
                                });
                        });
                }
                AppTab::Settings => {
                    egui::ScrollArea::vertical()
                        .auto_shrink([false; 2])
                        .id_salt("settings_scroll")
                        .show(ui, |ui| {
                            settings_tab::draw_settings_tab(self, ui);
                        });
                }
            }
        });
    }
}

// ─── Popup helper widgets ─────────────────────────────────────────────────────

fn popup_item(
    ui: &mut egui::Ui,
    icon: &str,
    label: &str,
    text_color: egui::Color32,
    hover_bg: egui::Color32,
) -> egui::Response {
    let desired_size = egui::vec2(ui.available_width(), 28.0);
    let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::click());
    if ui.is_rect_visible(rect) {
        let bg = if response.hovered() {
            hover_bg
        } else {
            egui::Color32::TRANSPARENT
        };
        ui.painter()
            .rect_filled(rect, egui::CornerRadius::same(6), bg);
        ui.painter().text(
            rect.min + egui::vec2(20.0, rect.height() / 2.0),
            egui::Align2::CENTER_CENTER,
            icon,
            egui::FontId::proportional(13.0),
            text_color,
        );
        ui.painter().text(
            rect.min + egui::vec2(36.0, rect.height() / 2.0),
            egui::Align2::LEFT_CENTER,
            label,
            egui::FontId::proportional(13.0),
            text_color,
        );
    }
    response
}

fn popup_section(ui: &mut egui::Ui, label: &str, text_color: egui::Color32) {
    let desired_size = egui::vec2(ui.available_width(), 24.0);
    let (rect, _) = ui.allocate_exact_size(desired_size, egui::Sense::hover());
    if ui.is_rect_visible(rect) {
        ui.painter().text(
            rect.min + egui::vec2(10.0, rect.height() / 2.0),
            egui::Align2::LEFT_CENTER,
            label,
            egui::FontId::proportional(11.0),
            text_color.linear_multiply(0.5),
        );
    }
}

fn popup_empty(ui: &mut egui::Ui, label: &str, text_color: egui::Color32) {
    let desired_size = egui::vec2(ui.available_width(), 24.0);
    let (rect, _) = ui.allocate_exact_size(desired_size, egui::Sense::hover());
    if ui.is_rect_visible(rect) {
        ui.painter().text(
            rect.min + egui::vec2(36.0, rect.height() / 2.0),
            egui::Align2::LEFT_CENTER,
            label,
            egui::FontId::proportional(12.0),
            text_color.linear_multiply(0.4),
        );
    }
}

fn popup_separator(ui: &mut egui::Ui, color: egui::Color32) {
    let desired_size = egui::vec2(ui.available_width(), 9.0);
    let (rect, _) = ui.allocate_exact_size(desired_size, egui::Sense::hover());
    if ui.is_rect_visible(rect) {
        let y = rect.center().y;
        ui.painter().line_segment(
            [
                egui::pos2(rect.min.x + 10.0, y),
                egui::pos2(rect.max.x - 10.0, y),
            ],
            egui::Stroke::new(1.0, color),
        );
    }
}

// ─── Native window hide ───────────────────────────────────────────────────────

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
