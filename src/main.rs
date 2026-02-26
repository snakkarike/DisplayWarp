#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod models;
mod monitor;
mod tray;
mod ui;
mod window;

use eframe::egui;

fn main() -> eframe::Result {
    // Create the tray icon on the main thread (required by Win32).
    let tray_items = tray::create_tray();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([720.0, 620.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Display Warp",
        options,
        Box::new(move |_| {
            let mut app = app::WindowManagerApp::default();
            app.tray = Some(tray_items);
            Ok(Box::new(app))
        }),
    )
}
