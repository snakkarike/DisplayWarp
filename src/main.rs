#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod audio;
mod models;
mod monitor;
mod server;
mod svg_render;
mod tray;
mod ui;
mod window;

use eframe::egui;
use windows::Win32::System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx};

fn main() -> eframe::Result {
<<<<<<< Updated upstream
    // 1. Set DPI Awareness (Per-Monitor V2) so we see actual physical pixels
    // instead of virtualized scaled coordinates.
    unsafe {
        use windows::Win32::UI::HiDpi::{
            DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetProcessDpiAwarenessContext,
        };
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }

    // Start the Web Bridge server in a background thread.
    std::thread::spawn(|| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(server::start_server());
    });

=======
    // Initialize COM for audio device enumeration/switching.
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
    }
>>>>>>> Stashed changes
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
            .with_inner_size([980.0, 740.0])
            .with_min_inner_size([960.0, 720.0])
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
                egui::FontData::from_static(include_bytes!("../assets/fonts/Geist-Regular.ttf"))
                    .into(),
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
