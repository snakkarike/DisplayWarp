#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod models;
mod monitor;
mod tray;
mod ui;
mod window;

use eframe::egui;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([720.0, 620.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Display Warp",
        options,
        Box::new(move |cc| {
            // Register Phosphor icon font.
            let mut fonts = egui::FontDefinitions::default();
            egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
            cc.egui_ctx.set_fonts(fonts);

            let mut app = app::WindowManagerApp::default();
            let tray_items = tray::create_tray(app.watcher_running.clone());
            app.tray = Some(tray_items);
            Ok(Box::new(app))
        }),
    )
}
