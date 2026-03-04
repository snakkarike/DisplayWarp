pub mod display_tab;
pub mod helpers;
pub mod log_tab;
pub mod monitor_preview;
pub mod quick_warp_tab;
pub mod settings_tab;
pub mod warp_tab;

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
                self.watcher_running
                    .store(false, std::sync::atomic::Ordering::Relaxed);
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

        // ── Central: everything else ─────────────────────────────────────
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_space(2.0);

            // Tab Bar with Logo
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
                            egui::vec2(146.0, 22.0), // Scaled down slightly to fit inline natively
                        ));
                        ui.add_space(8.0);
                    }

                    // Apply purple color for active tab background
                    ui.visuals_mut().selection.bg_fill = if self.dark_mode {
                        egui::Color32::from_rgb(76, 29, 149)
                    } else {
                        egui::Color32::from_rgb(139, 92, 246)
                    };

                    // Force the active selection text color to white so it contrasts with the purple background
                    ui.visuals_mut().selection.stroke.color = egui::Color32::WHITE;

                    ui.selectable_value(&mut self.current_tab, AppTab::QuickWarp, "Quick Warp");
                    ui.selectable_value(&mut self.current_tab, AppTab::Warp, "Warp");
                    ui.selectable_value(&mut self.current_tab, AppTab::Display, "Display");
                    ui.selectable_value(&mut self.current_tab, AppTab::Log, "Log");
                    ui.selectable_value(&mut self.current_tab, AppTab::Settings, "Settings");
                });
            });
            ui.add_space(2.0);
            ui.separator();
            ui.add_space(8.0);

            // Tab Content
            match self.current_tab {
                AppTab::QuickWarp => {
                    egui::ScrollArea::vertical()
                        .auto_shrink([false; 2])
                        .id_salt("quick_warp_scroll")
                        .show(ui, |ui| {
                            quick_warp_tab::draw_quick_warp_tab(self, ui);
                        });
                }
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
