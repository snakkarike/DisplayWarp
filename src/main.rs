#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod audio;
mod models;
mod monitor;
mod svg_render;
mod tray;
mod ui;
mod window;

use eframe::egui;
use windows::Win32::System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx};

fn main() -> eframe::Result {
    // 1. Set DPI Awareness (Per-Monitor V2) so we see actual physical pixels
    // instead of virtualized scaled coordinates.
    unsafe {
        use windows::Win32::UI::HiDpi::{
            DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetProcessDpiAwarenessContext,
        };
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }

    // Initialize COM for audio device enumeration/switching.
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
    }
    // Decode the PNG icon for the window titlebar.
    let (icon_rgba, w, h) =
        svg_render::png_to_rgba(include_bytes!("../assets/DisplayWarpIconTitlebar.png"));
    let icon = egui::IconData {
        rgba: icon_rgba,
        width: w,
        height: h,
    };

    // Pre-load data to check for "Start Minimized" configuration
    let mut start_visible = true;
    if let Ok(bytes) = std::fs::read(app::WindowManagerApp::get_config_path())
        && let Ok(decoded) = serde_json::from_slice::<models::SavedData>(&bytes)
    {
        start_visible = !decoded.start_minimized;
    }

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_visible(start_visible)
            .with_inner_size([980.0, 960.0])
            .with_min_inner_size([960.0, 960.0])
            .with_icon(std::sync::Arc::new(icon)),
        ..Default::default()
    };
    eframe::run_native(
        "DisplayWarp",
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

            // Load Application Header Logos
            let load_tex = |bytes: &[u8], name: &str| -> Option<egui::TextureHandle> {
                if let Ok(img) = image::load_from_memory(bytes) {
                    let rgba = img.to_rgba8();
                    let pixels = rgba.as_flat_samples();
                    let color_image = egui::ColorImage::from_rgba_unmultiplied(
                        [img.width() as _, img.height() as _],
                        pixels.as_slice(),
                    );
                    Some(
                        cc.egui_ctx
                            .load_texture(name, color_image, egui::TextureOptions::LINEAR),
                    )
                } else {
                    None
                }
            };

            app.logo_texture = load_tex(
                include_bytes!("../assets/DisplayWarpLogoBlack.png"),
                "logo-dark",
            );
            app.logo_texture_white = load_tex(
                include_bytes!("../assets/DisplayWarpLogoWhite.png"),
                "logo-light",
            );

            let tray_items = tray::create_tray(app.watcher_running.clone());
            app.tray = Some(tray_items);
            Ok(Box::new(app))
        }),
    )
}
