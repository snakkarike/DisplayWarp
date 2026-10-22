use eframe::egui;
use egui_phosphor::regular;

use crate::app::WindowManagerApp;

pub fn draw_shortcuts_tab(app: &mut WindowManagerApp, ui: &mut egui::Ui, available_h: f32) {
    let total_width = ui.available_width();
    let col_height = available_h - 8.0;

    ui.horizontal(|ui| {
        // ── Col 1: Live Desktop Shortcuts & Save Form (60% width) ───────
        ui.vertical(|ui| {
            ui.set_width(total_width * 0.60 - 4.0);
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

                    // Header
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.label(
                                egui::RichText::new(format!(
                                    "{} Live Desktop Shortcuts",
                                    regular::DESKTOP
                                ))
                                .size(16.0)
                                .strong(),
                            );
                            ui.label(
                                egui::RichText::new(
                                    "Current positions of shortcuts and icons across your monitors.",
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
                                .on_hover_text("Re-scan desktop shortcuts from Windows Explorer")
                                .clicked()
                            {
                                app.refresh_desktop_shortcuts();
                            }

                            let cat_btn = egui::Button::new(format!("{} Categories", regular::TAG))
                                .fill(if app.managing_categories {
                                    if app.dark_mode {
                                        egui::Color32::from_rgb(109, 40, 217)
                                    } else {
                                        egui::Color32::from_rgb(124, 58, 237)
                                    }
                                } else {
                                    egui::Color32::TRANSPARENT
                                });

                            if ui
                                .add(cat_btn)
                                .on_hover_text("Manage custom categories and color codes")
                                .clicked()
                            {
                                app.managing_categories = !app.managing_categories;
                            }
                        });
                    });

                    ui.add_space(8.0);

                    // Category Manager Panel (Collapsible)
                    if app.managing_categories {
                        egui::Frame::group(ui.style())
                            .inner_margin(egui::Margin::same(12))
                            .corner_radius(egui::CornerRadius::same(8))
                            .fill(if app.dark_mode {
                                egui::Color32::from_rgb(30, 30, 30)
                            } else {
                                egui::Color32::from_rgb(243, 244, 246)
                            })
                            .stroke(egui::Stroke::new(
                                1.0_f32,
                                if app.dark_mode {
                                    egui::Color32::from_rgb(60, 60, 60)
                                } else {
                                    egui::Color32::from_rgb(209, 213, 219)
                                },
                            ))
                            .show(ui, |ui| {
                                ui.set_width(ui.available_width());

                                ui.horizontal(|ui| {
                                    ui.label(
                                        egui::RichText::new(format!("{} Manage Shortcut Categories", regular::TAG))
                                            .strong()
                                            .size(14.0),
                                    );
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        if ui.button(regular::X).on_hover_text("Close").clicked() {
                                            app.managing_categories = false;
                                        }
                                    });
                                });

                                ui.add_space(6.0);

                                let categories = app.data.lock().shortcut_categories.clone();
                                let assignments = app.data.lock().shortcut_category_assignments.clone();

                                if categories.is_empty() {
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            egui::RichText::new("No categories defined yet.")
                                                .small()
                                                .color(egui::Color32::GRAY),
                                        );
                                        if ui
                                            .button(format!("{} Load Suggested Categories", regular::SPARKLE))
                                            .clicked()
                                        {
                                            app.load_suggested_categories();
                                        }
                                    });
                                } else {
                                    ui.horizontal_wrapped(|ui| {
                                        ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
                                        let mut cat_to_delete: Option<String> = None;
                                        for cat in &categories {
                                            let count = assignments.values().filter(|v| *v == &cat.id).count();
                                            let bg_color = cat.color32().linear_multiply(0.2);
                                            let border_color = cat.color32();

                                            egui::Frame::new()
                                                .fill(bg_color)
                                                .stroke(egui::Stroke::new(1.0_f32, border_color))
                                                .corner_radius(egui::CornerRadius::same(6))
                                                .inner_margin(egui::Margin::symmetric(8, 4))
                                                .show(ui, |ui| {
                                                    ui.horizontal(|ui| {
                                                        ui.label(
                                                            egui::RichText::new("●").color(cat.color32()),
                                                        );
                                                        ui.label(
                                                            egui::RichText::new(&cat.name)
                                                                .strong()
                                                                .small(),
                                                        );
                                                        ui.label(
                                                            egui::RichText::new(format!("({})", count))
                                                                .small()
                                                                .color(egui::Color32::GRAY),
                                                        );
                                                        if ui
                                                            .small_button(regular::TRASH)
                                                            .on_hover_text(format!("Delete category '{}'", cat.name))
                                                            .clicked()
                                                        {
                                                            cat_to_delete = Some(cat.id.clone());
                                                        }
                                                    });
                                                });
                                        }
                                        if let Some(id) = cat_to_delete {
                                            app.delete_shortcut_category(&id);
                                        }
                                    });
                                }

                                ui.add_space(8.0);
                                ui.separator();
                                ui.add_space(6.0);

                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = 6.0;
                                    ui.label(egui::RichText::new("New Category:").small().strong());

                                    let resp = ui.add_sized(
                                        [120.0, 26.0],
                                        egui::TextEdit::singleline(&mut app.new_category_name)
                                            .hint_text("Category name...")
                                            .margin(egui::Margin::symmetric(8, 4)),
                                    );

                                    let enter = resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));

                                    // Palette swatches
                                    for &(name, hex) in crate::models::PRESET_CATEGORY_COLORS {
                                        let color = crate::models::parse_hex_color(hex).unwrap_or(egui::Color32::WHITE);
                                        let is_active = app.new_category_color == hex;

                                        let (rect, response) = ui.allocate_exact_size(egui::vec2(18.0, 18.0), egui::Sense::click());
                                        if response.clicked() {
                                            app.new_category_color = hex.to_string();
                                        }
                                        let painter = ui.painter();
                                        painter.circle_filled(rect.center(), 7.0, color);
                                        if is_active {
                                            painter.circle_stroke(
                                                rect.center(),
                                                9.0,
                                                egui::Stroke::new(2.0_f32, if app.dark_mode { egui::Color32::WHITE } else { egui::Color32::BLACK }),
                                            );
                                        }
                                        response.on_hover_text(name);
                                    }

                                    let can_add = !app.new_category_name.trim().is_empty();
                                    if ui
                                        .add_enabled(can_add, egui::Button::new(format!("{} Add", regular::PLUS)))
                                        .clicked() || (enter && can_add)
                                    {
                                        let name = app.new_category_name.clone();
                                        let color = app.new_category_color.clone();
                                        if app.add_shortcut_category(&name, &color) {
                                            app.new_category_name.clear();
                                        }
                                    }
                                });
                            });
                        ui.add_space(8.0);
                    }

                    ui.add_space(8.0);

                    // Save Current Layout Card
                    egui::Frame::group(ui.style())
                        .inner_margin(egui::Margin::same(10))
                        .corner_radius(egui::CornerRadius::same(6))
                        .fill(if app.dark_mode {
                            egui::Color32::from_rgb(34, 34, 34)
                        } else {
                            egui::Color32::from_rgb(241, 245, 249)
                        })
                        .stroke(egui::Stroke::new(
                            1.0_f32,
                            if app.dark_mode {
                                egui::Color32::from_rgb(52, 52, 52)
                            } else {
                                egui::Color32::from_rgb(226, 232, 240)
                            },
                        ))
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.label(
                                egui::RichText::new(format!(
                                    "{} Save Current Shortcut Layout",
                                    regular::FLOPPY_DISK
                                ))
                                .strong(),
                            );
                            ui.label(
                                egui::RichText::new(
                                    "Capture the current (X, Y) coordinates of all desktop shortcuts.",
                                )
                                .small()
                                .color(if app.dark_mode {
                                    egui::Color32::from_gray(140)
                                } else {
                                    egui::Color32::from_gray(100)
                                }),
                            );
                            ui.add_space(6.0);

                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 8.0;
                                let btn_save_w = 88.0;
                                let btn_snap_w = 108.0;
                                let item_h = 28.0;
                                let spacing = ui.spacing().item_spacing.x;
                                let available = ui.available_width();
                                let text_edit_w = (available - btn_save_w - btn_snap_w - (spacing * 2.0)).max(120.0);

                                let is_empty = app.new_shortcut_layout_name.trim().is_empty();

                                let response = ui.add_sized(
                                    [text_edit_w, item_h],
                                    egui::TextEdit::singleline(&mut app.new_shortcut_layout_name)
                                        .hint_text("Layout name (e.g., Gaming, Work, Clean)")
                                        .desired_width(text_edit_w)
                                        .margin(egui::Margin::symmetric(10, 6)),
                                );

                                let enter_pressed = response.lost_focus()
                                    && ui.input(|i| i.key_pressed(egui::Key::Enter));

                                let save_btn = egui::Button::new(
                                    egui::RichText::new(format!(
                                        "{} Save",
                                        regular::FLOPPY_DISK
                                    ))
                                    .color(if is_empty {
                                        egui::Color32::from_gray(140)
                                    } else {
                                        egui::Color32::WHITE
                                    }),
                                )
                                .fill(if is_empty {
                                    if app.dark_mode {
                                        egui::Color32::from_rgb(45, 45, 45)
                                    } else {
                                        egui::Color32::from_rgb(220, 225, 230)
                                    }
                                } else {
                                    if app.dark_mode {
                                        egui::Color32::from_rgb(109, 40, 217)
                                    } else {
                                        egui::Color32::from_rgb(124, 58, 237)
                                    }
                                });

                                let save_clicked = ui
                                    .add_sized([btn_save_w, item_h], save_btn)
                                    .on_hover_text(if is_empty {
                                        "Enter a layout name to save"
                                    } else {
                                        "Save desktop shortcuts under this name"
                                    })
                                    .clicked();

                                if (save_clicked || enter_pressed) && !is_empty {
                                    let name = app.new_shortcut_layout_name.clone();
                                    app.save_shortcut_layout(&name);
                                    app.new_shortcut_layout_name.clear();
                                }

                                if ui
                                    .add_sized(
                                        [btn_snap_w, item_h],
                                        egui::Button::new(format!(
                                            "{} Snapshot",
                                            regular::CAMERA
                                        )),
                                    )
                                    .on_hover_text("Save instantly with automatic timestamp")
                                    .clicked()
                                {
                                    let name = format!(
                                        "Snapshot {}",
                                        crate::desktop_icons::current_timestamp_string()
                                    );
                                    app.save_shortcut_layout(&name);
                                }
                            });
                        });

                    ui.add_space(10.0);

                    // Filtered list of shortcuts
                    let query = app.shortcut_search_query.trim().to_lowercase();
                    let categories = app.data.lock().shortcut_categories.clone();

                    let mut filtered: Vec<_> = app
                        .live_desktop_shortcuts
                        .iter()
                        .filter(|icon| {
                            let text_match = query.is_empty() || icon.name.to_lowercase().contains(&query);
                            let mon_match = match app.shortcut_monitor_filter {
                                None => true,
                                Some(mon_idx) => {
                                    if let Some(m) = app.monitors.get(mon_idx) {
                                        icon.x >= m.rect.left
                                            && icon.x < m.rect.right
                                            && icon.y >= m.rect.top
                                            && icon.y < m.rect.bottom
                                    } else {
                                        true
                                    }
                                }
                            };
                            let cat_match = match &app.shortcut_category_filter {
                                None => true,
                                Some(filter_id) => {
                                    let assigned = app.get_shortcut_category(&icon.name);
                                    if filter_id == "uncategorized" {
                                        assigned.is_none()
                                    } else {
                                        assigned.as_ref().map(|c| &c.id) == Some(filter_id)
                                    }
                                }
                            };
                            text_match && mon_match && cat_match
                        })
                        .collect();

                    match app.shortcut_sort_mode {
                        crate::app::ShortcutSortMode::Default => {}
                        crate::app::ShortcutSortMode::NameAsc => {
                            filtered.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
                        }
                        crate::app::ShortcutSortMode::NameDesc => {
                            filtered.sort_by(|a, b| b.name.to_lowercase().cmp(&a.name.to_lowercase()));
                        }
                    }

                    // Search, Monitor Filter & Category Filter Bar
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 8.0;
                        ui.spacing_mut().interact_size.y = 28.0;

                        let total_shortcuts = app.live_desktop_shortcuts.len();
                        let filtered_count = filtered.len();
                        let stats_text = if filtered_count == total_shortcuts {
                            format!("{} Shortcuts", total_shortcuts)
                        } else {
                            format!("{}/{} Shortcuts", filtered_count, total_shortcuts)
                        };

                        ui.label(
                            egui::RichText::new(stats_text)
                                .strong()
                                .color(if app.dark_mode {
                                    egui::Color32::from_rgb(196, 181, 253)
                                } else {
                                    egui::Color32::from_rgb(109, 40, 217)
                                }),
                        );

                        // Sort selector dropdown
                        let current_sort_label = match app.shortcut_sort_mode {
                            crate::app::ShortcutSortMode::Default => {
                                format!("{} Default", regular::ARROWS_DOWN_UP)
                            }
                            crate::app::ShortcutSortMode::NameAsc => {
                                format!("{} Name (A-Z)", regular::SORT_ASCENDING)
                            }
                            crate::app::ShortcutSortMode::NameDesc => {
                                format!("{} Name (Z-A)", regular::SORT_DESCENDING)
                            }
                        };

                        let sort_combo_resp = egui::ComboBox::from_id_salt("live_shortcut_sort_mode_combo")
                            .selected_text(current_sort_label)
                            .width(120.0)
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut app.shortcut_sort_mode,
                                    crate::app::ShortcutSortMode::Default,
                                    format!("{} Default (Position)", regular::ARROWS_DOWN_UP),
                                );
                                ui.selectable_value(
                                    &mut app.shortcut_sort_mode,
                                    crate::app::ShortcutSortMode::NameAsc,
                                    format!("{} Name (A-Z)", regular::SORT_ASCENDING),
                                );
                                ui.selectable_value(
                                    &mut app.shortcut_sort_mode,
                                    crate::app::ShortcutSortMode::NameDesc,
                                    format!("{} Name (Z-A)", regular::SORT_DESCENDING),
                                );
                            });
                        sort_combo_resp.response.on_hover_text("Sort shortcuts: Default desktop position, Name (A-Z), or Name (Z-A)");

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.spacing_mut().item_spacing.x = 8.0;
                            ui.spacing_mut().interact_size.y = 28.0;

                            let item_h = 28.0;
                            let search_w = 110.0;
                            let combo_w = 125.0;
                            let cat_combo_w = 125.0;

                            // Text search filter on the far right
                            ui.add_sized(
                                [search_w, item_h],
                                egui::TextEdit::singleline(&mut app.shortcut_search_query)
                                    .hint_text(format!("{} Filter...", regular::MAGNIFYING_GLASS))
                                    .desired_width(search_w)
                                    .margin(egui::Margin::symmetric(10, 6)),
                            );

                            // Monitor selector dropdown
                            let current_mon_label = match app.shortcut_monitor_filter {
                                None => format!("{} All Monitors", regular::DESKTOP),
                                Some(idx) => {
                                    if let Some(m) = app.monitors.get(idx) {
                                        let name = m
                                            .hardware_name
                                            .as_deref()
                                            .unwrap_or(m.device_name.as_str());
                                        format!("{} Mon {}: {}", regular::DESKTOP, idx + 1, name)
                                    } else {
                                        format!("{} Monitor {}", regular::DESKTOP, idx + 1)
                                    }
                                }
                            };

                            egui::ComboBox::from_id_salt("live_shortcut_monitor_filter_combo")
                                .selected_text(current_mon_label)
                                .width(combo_w)
                                .show_ui(ui, |ui| {
                                    let is_all = app.shortcut_monitor_filter.is_none();
                                    if ui
                                        .selectable_label(
                                            is_all,
                                            format!("{} All Monitors", regular::DESKTOP),
                                        )
                                        .clicked()
                                    {
                                        app.shortcut_monitor_filter = None;
                                    }

                                    for (i, m) in app.monitors.iter().enumerate() {
                                        let is_sel = app.shortcut_monitor_filter == Some(i);
                                        let name = m
                                            .hardware_name
                                            .as_deref()
                                            .unwrap_or(m.device_name.as_str());
                                        let label = format!("{} Mon {}: {}", regular::DESKTOP, i + 1, name);
                                        if ui.selectable_label(is_sel, label).clicked() {
                                            app.shortcut_monitor_filter = Some(i);
                                        }
                                    }
                                });

                            // Category selector dropdown
                            let current_cat_label = match &app.shortcut_category_filter {
                                None => format!("{} All Categories", regular::TAG),
                                Some(id) => {
                                    if id == "uncategorized" {
                                        format!("{} Untagged", regular::TAG)
                                    } else if let Some(cat) = categories.iter().find(|c| &c.id == id) {
                                        format!("● {}", cat.name)
                                    } else {
                                        format!("{} Category", regular::TAG)
                                    }
                                }
                            };

                            egui::ComboBox::from_id_salt("live_shortcut_category_filter_combo")
                                .selected_text(current_cat_label)
                                .width(cat_combo_w)
                                .show_ui(ui, |ui| {
                                    let is_all = app.shortcut_category_filter.is_none();
                                    if ui
                                        .selectable_label(
                                            is_all,
                                            format!("{} All Categories", regular::TAG),
                                        )
                                        .clicked()
                                    {
                                        app.shortcut_category_filter = None;
                                    }

                                    for cat in &categories {
                                        let is_sel = app.shortcut_category_filter.as_ref() == Some(&cat.id);
                                        let label = egui::RichText::new(format!("● {}", cat.name)).color(cat.color32());
                                        if ui.selectable_label(is_sel, label).clicked() {
                                            app.shortcut_category_filter = Some(cat.id.clone());
                                        }
                                    }

                                    if ui
                                        .selectable_label(
                                            app.shortcut_category_filter.as_deref() == Some("uncategorized"),
                                            format!("{} Untagged", regular::TAG),
                                        )
                                        .clicked()
                                    {
                                        app.shortcut_category_filter = Some("uncategorized".to_string());
                                    }
                                });
                        });
                    });

                    ui.add_space(8.0);

                    let mut to_update_category: Option<(String, Option<String>)> = None;

                    egui::ScrollArea::vertical()
                        .id_salt("live_shortcuts_list_scroll")
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            if filtered.is_empty() {
                                ui.vertical_centered(|ui| {
                                    ui.add_space(30.0);
                                    ui.label(
                                        egui::RichText::new(if app.live_desktop_shortcuts.is_empty() {
                                            "No desktop shortcuts detected. Click Refresh above."
                                        } else {
                                            "No shortcuts matched your search, monitor, or category filter."
                                        })
                                        .color(egui::Color32::GRAY),
                                    );
                                });
                            } else {
                                for (idx, icon) in filtered.iter().enumerate() {
                                    // Determine which monitor this icon is on
                                    let mon_label = app
                                        .monitors
                                        .iter()
                                        .find(|m| {
                                            icon.x >= m.rect.left
                                                && icon.x < m.rect.right
                                                && icon.y >= m.rect.top
                                                && icon.y < m.rect.bottom
                                        })
                                        .map(|m| {
                                            m.hardware_name
                                                .as_deref()
                                                .unwrap_or(m.device_name.as_str())
                                        })
                                        .unwrap_or("Desktop");

                                    let current_cat = app.get_shortcut_category(&icon.name);

                                    let is_even = idx % 2 == 0;
                                    let row_bg = if app.dark_mode {
                                        if is_even {
                                            egui::Color32::from_rgb(22, 22, 22)
                                        } else {
                                            egui::Color32::from_rgb(28, 28, 28)
                                        }
                                    } else {
                                        if is_even {
                                            egui::Color32::from_rgb(255, 255, 255)
                                        } else {
                                            egui::Color32::from_rgb(245, 247, 250)
                                        }
                                    };

                                    egui::Frame::new()
                                        .fill(row_bg)
                                        .inner_margin(egui::Margin::symmetric(8, 5))
                                        .corner_radius(egui::CornerRadius::same(4))
                                        .show(ui, |ui| {
                                            ui.horizontal(|ui| {
                                                let icon_color = current_cat.as_ref().map(|c| c.color32()).unwrap_or(egui::Color32::GRAY);
                                                ui.label(
                                                    egui::RichText::new(format!(
                                                        "{}",
                                                        regular::APP_WINDOW
                                                    ))
                                                    .color(icon_color),
                                                );
                                                ui.label(
                                                    egui::RichText::new(&icon.name)
                                                        .strong(),
                                                );

                                                ui.with_layout(
                                                    egui::Layout::right_to_left(egui::Align::Center),
                                                    |ui| {
                                                        ui.label(
                                                            egui::RichText::new(format!(
                                                                "({}, {})",
                                                                icon.x, icon.y
                                                            ))
                                                            .monospace()
                                                            .small()
                                                            .color(egui::Color32::GRAY),
                                                        );
                                                        ui.label(
                                                            egui::RichText::new(format!(
                                                                "[{}]",
                                                                mon_label
                                                            ))
                                                            .small()
                                                            .color(if app.dark_mode {
                                                                egui::Color32::from_rgb(167, 139, 250)
                                                            } else {
                                                                egui::Color32::from_rgb(124, 58, 237)
                                                            }),
                                                        );

                                                        // Category dropdown pill
                                                        let cat_text = if let Some(cat) = &current_cat {
                                                            egui::RichText::new(format!("● {}", cat.name)).color(cat.color32()).small()
                                                        } else {
                                                            egui::RichText::new(format!("{} Tag", regular::TAG)).color(egui::Color32::GRAY).small()
                                                        };

                                                        egui::ComboBox::from_id_salt(format!("cat_sel_{}_{}", idx, icon.name))
                                                            .selected_text(cat_text)
                                                            .width(80.0)
                                                            .show_ui(ui, |ui| {
                                                                if ui.selectable_label(current_cat.is_none(), "Untagged").clicked() {
                                                                    to_update_category = Some((icon.name.clone(), None));
                                                                }
                                                                for cat in &categories {
                                                                    let is_sel = current_cat.as_ref().map(|c| &c.id) == Some(&cat.id);
                                                                    let item_label = egui::RichText::new(format!("● {}", cat.name)).color(cat.color32());
                                                                    if ui.selectable_label(is_sel, item_label).clicked() {
                                                                        to_update_category = Some((icon.name.clone(), Some(cat.id.clone())));
                                                                    }
                                                                }
                                                            });
                                                    },
                                                );
                                            });
                                        });
                                }
                            }
                        });

                    if let Some((name, cat_id)) = to_update_category {
                        app.set_shortcut_category(&name, cat_id);
                    }
                });
        });

        // ── Col 2: Saved Shortcut Layouts (40% width) ──────────────────
        ui.vertical(|ui| {
            ui.set_width(total_width * 0.40 - 8.0);
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
                        egui::RichText::new(format!(
                            "{} Saved Shortcut Layouts",
                            regular::BOOKMARK_SIMPLE
                        ))
                        .size(16.0)
                        .strong(),
                    );
                    ui.label(
                        egui::RichText::new(
                            "Restore saved shortcut layouts across single or multi-monitors.",
                        )
                        .small()
                        .color(if app.dark_mode {
                            egui::Color32::from_gray(140)
                        } else {
                            egui::Color32::from_gray(100)
                        }),
                    );
                    ui.add_space(8.0);

                    let saved_layouts = app.data.lock().shortcut_layouts.clone();

                    if saved_layouts.is_empty() {
                        ui.vertical_centered(|ui| {
                            ui.add_space(40.0);
                            ui.label(
                                egui::RichText::new(format!("{}", regular::FOLDER_OPEN))
                                    .size(32.0)
                                    .color(egui::Color32::GRAY),
                            );
                            ui.add_space(8.0);
                            ui.label(
                                egui::RichText::new("No shortcut layouts saved yet.")
                                    .color(egui::Color32::GRAY),
                            );
                            ui.label(
                                egui::RichText::new(
                                    "Arrange shortcuts on your desktop, enter a name on the left, and click Save.",
                                )
                                .small()
                                .color(egui::Color32::GRAY),
                            );
                        });
                        return;
                    }

                    let mut to_restore: Option<usize> = None;
                    let mut to_remove: Option<usize> = None;
                    let mut to_move_up: Option<usize> = None;
                    let mut to_move_down: Option<usize> = None;

                    egui::ScrollArea::vertical()
                        .id_salt("saved_shortcut_layouts_scroll")
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            for (idx, layout) in saved_layouts.iter().enumerate() {
                                let is_viewing = app.viewing_shortcut_layout_idx == Some(idx);

                                egui::Frame::group(ui.style())
                                    .inner_margin(egui::Margin::same(10))
                                    .corner_radius(egui::CornerRadius::same(6))
                                    .fill(if app.dark_mode {
                                        egui::Color32::from_rgb(34, 34, 34)
                                    } else {
                                        egui::Color32::from_rgb(241, 245, 249)
                                    })
                                    .stroke(egui::Stroke::new(
                                        1.0_f32,
                                        if is_viewing {
                                            egui::Color32::from_rgb(139, 92, 246)
                                        } else if app.dark_mode {
                                            egui::Color32::from_rgb(50, 50, 50)
                                        } else {
                                            egui::Color32::from_rgb(226, 232, 240)
                                        },
                                    ))
                                    .show(ui, |ui| {
                                        ui.vertical(|ui| {
                                            // Top Row: Title + Reorder buttons
                                            ui.horizontal(|ui| {
                                                ui.label(
                                                    egui::RichText::new(&layout.name)
                                                        .size(14.0)
                                                        .strong(),
                                                );

                                                ui.with_layout(
                                                    egui::Layout::right_to_left(egui::Align::Center),
                                                    |ui| {
                                                        if ui.button(regular::CARET_DOWN).clicked() {
                                                            to_move_down = Some(idx);
                                                        }
                                                        if ui.button(regular::CARET_UP).clicked() {
                                                            to_move_up = Some(idx);
                                                        }
                                                    },
                                                );
                                            });

                                            // Sub-info: timestamp & icon count
                                            ui.horizontal(|ui| {
                                                ui.label(
                                                    egui::RichText::new(format!(
                                                        "{} shortcuts",
                                                        layout.icons.len()
                                                    ))
                                                    .small()
                                                    .color(if app.dark_mode {
                                                        egui::Color32::from_rgb(196, 181, 253)
                                                    } else {
                                                        egui::Color32::from_rgb(109, 40, 217)
                                                    }),
                                                );
                                                if !layout.created_at.is_empty() {
                                                    ui.label(
                                                        egui::RichText::new(format!(
                                                            "• {}",
                                                            layout.created_at
                                                        ))
                                                        .small()
                                                        .color(egui::Color32::GRAY),
                                                    );
                                                }
                                            });

                                            ui.add_space(6.0);

                                            // Action Buttons: Restore, View, Delete
                                            ui.horizontal(|ui| {
                                                let restore_btn = ui.add(
                                                    egui::Button::new(
                                                        egui::RichText::new(format!(
                                                            "{} Restore",
                                                            regular::PLAY
                                                        ))
                                                        .color(egui::Color32::WHITE),
                                                    )
                                                    .fill(if app.dark_mode {
                                                        egui::Color32::from_rgb(22, 101, 52)
                                                    } else {
                                                        egui::Color32::from_rgb(34, 197, 94)
                                                    }),
                                                );
                                                if restore_btn.clicked() {
                                                    to_restore = Some(idx);
                                                }

                                                let view_label = if is_viewing {
                                                    format!("{} Hide", regular::EYE_SLASH)
                                                } else {
                                                    format!("{} Details", regular::EYE)
                                                };
                                                if ui.button(view_label).clicked() {
                                                    if is_viewing {
                                                        app.viewing_shortcut_layout_idx = None;
                                                    } else {
                                                        app.viewing_shortcut_layout_idx = Some(idx);
                                                    }
                                                }

                                                if ui
                                                    .button(format!("{} Delete", regular::TRASH))
                                                    .clicked()
                                                {
                                                    to_remove = Some(idx);
                                                }
                                            });

                                            // Expanded items preview
                                            if is_viewing {
                                                ui.add_space(6.0);
                                                ui.separator();
                                                ui.add_space(4.0);

                                                let query = app.shortcut_search_query.trim().to_lowercase();
                                                let categories = app.data.lock().shortcut_categories.clone();
                                                let assignments = app.data.lock().shortcut_category_assignments.clone();
                                                let mut layout_filtered: Vec<_> = layout
                                                    .icons
                                                    .iter()
                                                    .filter(|item| {
                                                        let text_match = query.is_empty()
                                                            || item.name.to_lowercase().contains(&query);
                                                        let mon_match = match app.shortcut_monitor_filter {
                                                            None => true,
                                                            Some(mon_idx) => {
                                                                if let Some(m) = app.monitors.get(mon_idx) {
                                                                    item.x >= m.rect.left
                                                                        && item.x < m.rect.right
                                                                        && item.y >= m.rect.top
                                                                        && item.y < m.rect.bottom
                                                                } else {
                                                                    true
                                                                }
                                                            }
                                                        };
                                                        let cat_id = item.category_id.as_deref()
                                                            .or_else(|| assignments.get(&item.name).map(|s| s.as_str()));
                                                        let cat_match = match &app.shortcut_category_filter {
                                                            None => true,
                                                            Some(filter_id) => {
                                                                if filter_id == "uncategorized" {
                                                                    cat_id.is_none()
                                                                } else {
                                                                    cat_id == Some(filter_id.as_str())
                                                                }
                                                            }
                                                        };
                                                        text_match && mon_match && cat_match
                                                    })
                                                    .collect();

                                                match app.shortcut_sort_mode {
                                                    crate::app::ShortcutSortMode::Default => {}
                                                    crate::app::ShortcutSortMode::NameAsc => {
                                                        layout_filtered.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
                                                    }
                                                    crate::app::ShortcutSortMode::NameDesc => {
                                                        layout_filtered.sort_by(|a, b| b.name.to_lowercase().cmp(&a.name.to_lowercase()));
                                                    }
                                                }

                                                ui.horizontal(|ui| {
                                                    ui.label(
                                                        egui::RichText::new("Saved Shortcut Coordinates:")
                                                            .small()
                                                            .strong(),
                                                    );
                                                    if layout_filtered.len() != layout.icons.len() {
                                                        ui.label(
                                                            egui::RichText::new(format!(
                                                                "({} of {})",
                                                                layout_filtered.len(),
                                                                layout.icons.len()
                                                            ))
                                                            .small()
                                                            .color(egui::Color32::GRAY),
                                                        );
                                                    }
                                                });
                                                egui::ScrollArea::vertical()
                                                    .id_salt(format!("layout_preview_scroll_{}", idx))
                                                    .max_height(140.0)
                                                    .show(ui, |ui| {
                                                        if layout_filtered.is_empty() {
                                                            ui.label(
                                                                egui::RichText::new("No shortcuts match current filter.")
                                                                    .small()
                                                                    .color(egui::Color32::GRAY),
                                                            );
                                                        } else {
                                                            for item in layout_filtered {
                                                                let mon_label = app
                                                                    .monitors
                                                                    .iter()
                                                                    .find(|m| {
                                                                        item.x >= m.rect.left
                                                                            && item.x < m.rect.right
                                                                            && item.y >= m.rect.top
                                                                            && item.y < m.rect.bottom
                                                                    })
                                                                    .map(|m| {
                                                                        m.hardware_name
                                                                            .as_deref()
                                                                            .unwrap_or(m.device_name.as_str())
                                                                    })
                                                                    .unwrap_or("Desktop");

                                                                let item_cat = item.category_id.as_deref()
                                                                    .or_else(|| assignments.get(&item.name).map(|s| s.as_str()))
                                                                    .and_then(|id| categories.iter().find(|c| c.id == id));

                                                                ui.horizontal(|ui| {
                                                                    let icon_color = item_cat.as_ref().map(|c| c.color32()).unwrap_or(egui::Color32::GRAY);
                                                                    ui.label(
                                                                        egui::RichText::new(format!("{}", regular::APP_WINDOW))
                                                                            .color(icon_color),
                                                                    );
                                                                    ui.label(
                                                                        egui::RichText::new(&item.name)
                                                                            .small(),
                                                                    );
                                                                    ui.with_layout(
                                                                        egui::Layout::right_to_left(
                                                                            egui::Align::Center,
                                                                        ),
                                                                        |ui| {
                                                                            ui.label(
                                                                                egui::RichText::new(
                                                                                    format!(
                                                                                        "({}, {})",
                                                                                        item.x, item.y
                                                                                    ),
                                                                                )
                                                                                .monospace()
                                                                                .small()
                                                                                .color(
                                                                                    egui::Color32::GRAY,
                                                                                ),
                                                                            );
                                                                            ui.label(
                                                                                egui::RichText::new(
                                                                                    format!(
                                                                                        "[{}]",
                                                                                        mon_label
                                                                                    ),
                                                                                )
                                                                                .small()
                                                                                .color(if app.dark_mode {
                                                                                    egui::Color32::from_rgb(167, 139, 250)
                                                                                } else {
                                                                                    egui::Color32::from_rgb(124, 58, 237)
                                                                                }),
                                                                            );
                                                                            if let Some(cat) = item_cat {
                                                                                ui.label(
                                                                                    egui::RichText::new(format!("● {}", cat.name))
                                                                                        .small()
                                                                                        .color(cat.color32()),
                                                                                );
                                                                            }
                                                                        },
                                                                    );
                                                                });
                                                            }
                                                        }
                                                    });
                                            }
                                        });
                                    });
                                ui.add_space(6.0);
                            }
                        });

                    // Handle actions
                    if let Some(idx) = to_restore {
                        app.restore_shortcut_layout(idx);
                    }

                    if let Some(idx) = to_remove {
                        app.delete_shortcut_layout(idx);
                        if app.viewing_shortcut_layout_idx == Some(idx) {
                            app.viewing_shortcut_layout_idx = None;
                        }
                    }

                    if let Some(i) = to_move_up {
                        if i > 0 {
                            app.data.lock().shortcut_layouts.swap(i, i - 1);
                            app.save_data();
                            app.refresh_tray();
                        }
                    }

                    if let Some(i) = to_move_down {
                        let len = app.data.lock().shortcut_layouts.len();
                        if i + 1 < len {
                            app.data.lock().shortcut_layouts.swap(i, i + 1);
                            app.save_data();
                            app.refresh_tray();
                        }
                    }
                });
        });
    });
}
