use tray_icon::menu::{Menu, MenuEvent, MenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

/// Build a 16×16 RGBA icon filled with a single color.
fn simple_icon(r: u8, g: u8, b: u8) -> Icon {
    let size = 16u32;
    let mut rgba = Vec::with_capacity((size * size * 4) as usize);
    for _ in 0..(size * size) {
        rgba.extend_from_slice(&[r, g, b, 255]);
    }
    Icon::from_rgba(rgba, size, size).expect("failed to create tray icon")
}

/// Items exposed so the UI can react to menu clicks.
pub struct TrayItems {
    pub show_id: tray_icon::menu::MenuId,
    pub quit_id: tray_icon::menu::MenuId,
    pub _tray: TrayIcon, // must stay alive
}

/// Create the system-tray icon and context menu.
/// Call this **once** on the main thread, before entering the event loop.
pub fn create_tray() -> TrayItems {
    let show = MenuItem::new("Show", true, None);
    let quit = MenuItem::new("Quit", true, None);
    let show_id = show.id().clone();
    let quit_id = quit.id().clone();

    let menu = Menu::new();
    let _ = menu.append(&show);
    let _ = menu.append(&quit);

    let icon = simple_icon(50, 200, 100); // our green accent

    let tray = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip("DisplayWarp")
        .with_icon(icon)
        .build()
        .expect("failed to build tray icon");

    TrayItems {
        show_id,
        quit_id,
        _tray: tray,
    }
}

/// Drain pending menu events. Returns `Some(id)` for each click.
pub fn poll_menu_event() -> Option<tray_icon::menu::MenuId> {
    MenuEvent::receiver()
        .try_recv()
        .ok()
        .map(|e| e.id().clone())
}
