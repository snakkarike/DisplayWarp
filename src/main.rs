#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod models;
mod monitor;
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
        Box::new(|_| Ok(Box::new(app::WindowManagerApp::default()))),
    )
}
