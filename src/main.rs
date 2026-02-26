#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod models;
mod monitor;
mod svg_render;
mod tray;
mod ui;
mod window;

use eframe::egui;

fn main() -> eframe::Result {
    // Decode the PNG icon for the window titlebar.
    let (icon_rgba, w, h) =
        svg_render::png_to_rgba(include_bytes!("../assets/DisplayWarpIcon.png"));
    let icon = egui::IconData {
        rgba: icon_rgba,
        width: w,
        height: h,
    };

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([720.0, 800.0])
            .with_min_inner_size([700.0, 600.0])
            .with_icon(std::sync::Arc::new(icon)),
        ..Default::default()
    };
    eframe::run_native(
        "Display Warp",
        options,
        Box::new(move |cc| {
            // Register fonts.
            let mut fonts = egui::FontDefinitions::default();

            // Geist Sans as primary proportional font.
            fonts.font_data.insert(
                "geist".into(),
                egui::FontData::from_static(include_bytes!("../assets/fonts/Geist-Regular.ttf")),
            );
            fonts
                .families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .insert(0, "geist".into());

            // Phosphor icons as fallback.
            egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);

            cc.egui_ctx.set_fonts(fonts);

            let mut app = app::WindowManagerApp::default();
            let tray_items = tray::create_tray(app.watcher_running.clone());
            app.tray = Some(tray_items);
            Ok(Box::new(app))
        }),
    )
}
