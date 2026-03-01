## v1.0.7

### ✨ New Features

- **Audio Device Selection**: Integrated audio device management (via the Audio-Mod branch) allowing you to target specific audio outputs when moving applications.

- **Tabbed Interface**: Removed the single continuous scroll view and implemented a clean, tabbed navigation system (`Warp`, `Monitors`, `Log`, `Settings`) to organize features better.

- **Independent Scroll Areas**: The top tab bar and status indicators are now globally pinned to the screen, while individual tab contents scroll independently.

- **Dynamic GitHub Changelog**: The application now automatically fetches the latest release notes directly from GitHub and renders them in rich Markdown format natively inside the app.

## 🎨 UI & UX Improvements

- **Settings & About Overhaul**: Redesigned the Settings tab into a two-column layout, splitting configurable options (like Theme toggling) from the "About DisplayWarp" information.

- **Embedded Graphics:** Added the native DisplayWarp logo inside the application's About section for a more premium look.

## 🛠️ Fixes & Optimizations

- **Window Placement Optimization**: Refactored the window movement loop ([move_to_monitor](cci:1://file:///g:/Github/DisplayWarp/src/window.rs:333:0-413:1) and [watch_window_on_monitor](cci:1://file:///g:/Github/DisplayWarp/src/window.rs:299:0-331:1)). Removed aggressive `SW_MAXIMIZE` behaviors and adjusted polling delays to prevent flickering or unwanted window state changes (especially useful for exclusive-fullscreen games).

- **Dependency Cleanup:** Trimmed down unused layout dependencies to reduce binary footprint and compilation times.

---

## 1.0.6

### Added

- **UI Grid**: New 3-column layout (Move Live Window | New Profile | Saved Profiles) that automatically expands to fill the window height.

- **Theme Toggle**: Switch between Light and Dark modes via the new toggle in the bottom bar.

- **Version Indicator**: Version number pinned to the bottom-left corner for quick reference.

- **Icon Metadata**: Embedded executable metadata (CompanyName, FileDescription, ProductVersion) using `winresource`.

### Changed

- **Major Dependency Updates**:
  - [windows](cci:1://file:///g:/Github/DisplayWarp/src/window.rs:342:0-355:1) crate: `0.58` → `0.62`

  - [eframe](cci:1://file:///g:/Github/DisplayWarp/src/ui/mod.rs:412:0-421:1) / `egui`: `0.29` → `0.33`

  - `egui-phosphor`: `0.7` → `0.11`

  - `tray-icon`: `0.19` → `0.21`

  - `rfd`: `0.14` → `0.17`

- **UI Refactor**:
  - Migrated styling from `.rounding()` to the modern `.corner_radius()` API.

  - Replaced deprecated `Frame::none()` with `Frame::NONE`.

  - Updated `Margin` and `CornerRadius` to use integer literals (e.g., `12` instead of `12.0`) following `egui` type changes.

  - Improved layout logic to ensure all main columns maintain equal height.

- **Windows API Modernization**:
  - Refactored `Win32` imports to use `windows::core::BOOL` (handling the removal of `BOOL/TRUE/FALSE` from `Foundation`).

  - Updated API call signatures for `IsWindow`, `SetWindowPos`, `EnumDisplayMonitors`, and `ChangeDisplaySettingsExW` to match [windows](cci:1://file:///g:/Github/DisplayWarp/src/window.rs:342:0-355:1) v0.62 requirements.

### Fixed

- **UI Scaling**: Optimized button labels and switched headers to vertical stacking to prevent overlap in narrow columns.

- **Rendering**: Corrected `rect_stroke` calls in the monitor preview by providing the required `StrokeKind` argument.

- **Warnings**: Resolved an `unused_assignments` compiler warning in the close dialog logic.

---

## v1.0.5

### 🚀 Enhancements

- **Auto-Refreshing Processes** — The live process list is now automatically fetched and populated when DisplayWarp starts, removing the need to manually click "Refresh Process List" on initial load.

- **Window Size Adjustments** — Increased the default window dimensions (720×1000) and minimum bounds (700×980) to ensure the new profile forms and live process menus fit comfortably on screen without getting cut off.

### 🔧 Fixes

- **Dropdown Height Consistency** — Enforced a fixed height for the Live Process dropdown menu. It will no longer occasionally truncate to only show 2 items at a time.

- **Empty List Protection** — The Live Process dropdown is now visually disabled and unclickable if no processes are detected, preventing unintended UI glitches before a refresh.

- **Disabled Persistent Windows (Temporary)** — The "Persistent Window" functionality has been temporarily disabled from the UI while an underlying tracking issue is investigated and resolved.

---

## v1.0.4

### 🚀 Enhancements

- **Auto-Refreshing Processes** — The live process list is now automatically fetched and populated when DisplayWarp starts, removing the need to manually click "Refresh Process List" on initial load.

- **Window Size Adjustments** — Increased the default window dimensions (720×900) and minimum bounds (700×750) to ensure the new profile forms and live process menus fit comfortably on screen without getting cut off.

### 🔧 Fixes

- **Dropdown Height Consistency** — Enforced a fixed height for the Live Process dropdown menu. It will no longer occasionally truncate to only show 2 items at a time.

- **Empty List Protection** — The Live Process dropdown is now visually disabled and unclickable if no processes are detected, preventing unintended UI glitches before a refresh.

- **Disabled Persistent Windows (Temporary)** — The "Persistent Window" functionality has been temporarily disabled from the UI while an underlying tracking issue is investigated and resolved.

---

## v1.0.3

### ✨ New Features

- **Custom Profile Names** — You can now assign a custom name to a profile when creating it. The name defaults to the executable file name if left blank.

- **Edit Profile Names** — Existing profiles can now be renamed directly from the edit menu.

- **Quick Add from Live Process** — Added a new **Create Profile** button to the "Move Live Window" section. This allows you to rapidly create a custom profile by simply selecting an active window from the dropdown and your desired monitor, automatically detecting and saving the underlying executable path.

---

## v1.0.2

### ✨ New Features

- **Close Confirmation Dialog** — Pressing the window close button now shows a centered popup with two options: **Minimize to Tray** or **Quit**. Includes a dimmed overlay behind the dialog for focus.

### 🎨 UI Polish

- **Geist Font** — Replaced the default egui font with Vercel's Geist Sans for a cleaner, modern look. Embedded directly in the binary.

- **Phosphor Icons** — All UI icons replaced with crisp Phosphor vector icons (folder, play, trash, pencil, monitor, power, etc.) via the `egui-phosphor` crate.

- **Two-Column Flex Layout** — New Profiles + Move Live Window on the left, Saved Profiles on the right. Both columns stretch to full height with matching borders.

- **Move Live Window** relocated from a standalone bottom panel into the left column below Create Profile, separated by a divider.

- **Log Panel** pinned to the bottom of the window via a dedicated BottomPanel.

- **Saved Profiles** scroll area flex-expands with window height.

- **Button Padding** increased globally for more comfortable click targets.

- **Dropdown Heights** increased so all combobox popups show many items at once.

- **Process Icons** — Each live process entry in the dropdown now shows an app window icon prefix.

- **Ellipsis Truncation** — Long process names in the live window dropdown are truncated with `…` to prevent layout overflow.

### 📦 Dependencies Added

- `egui-phosphor 0.7` — Phosphor icon font for egui 0.29

### 🔧 Fixes

- Removed outer page scroll — content fits the window directly.

- Minimum window size increased to 700×600.

---

## v1.0.1

### ✨ New Features

- **Minimize to Tray** — Closing the window now hides it to the system tray instead of quitting. The app continues running in the background. Right-click the tray icon → **Show** to restore, **Quit** to exit.

- **Persistent Monitor Enforcement** — New per-profile **"Persistent Window"** toggle. When enabled, a background watcher checks every 3 seconds if the profiled window is on the correct monitor and moves it back automatically. Works even when the app is minimized to tray. Requires the Window Process field to be set.

### 🎨 UI Overhaul

- **Two-column layout** — New Profiles + Move Live Window on the left, Saved Profiles on the right. Saved Profiles panel flex-expands with window height.

- **Phosphor Icons** — Replaced emoji icons with crisp Phosphor vector icons throughout.

- **Color-coded monitor preview** — Green = selected target, Blue = primary monitor, Purple = other monitors. Resolution labels and monitor number badges on each display.

- **Monitor legend** — Green/blue dot legend below the preview for clarity.

- **Styled monitor selector** — Active monitor button highlighted in green.

- **Profile cards** — Display badge, exe path, process name, persistent toggle, and evenly-spaced Launch / Edit / Delete buttons.

- **Log panel** — Pinned to the bottom of the window, full-width status display.

### 🛡 Stability

- **Thread-safe profile data** — Profiles wrapped in `Arc<Mutex<SavedData>>` for safe access between the UI thread and the background watcher.

- **Stale HWND protection** — `IsWindow` checks before all Win32 API calls to prevent crashes during display layout changes.

- **Graceful monitor bounds** — Monitor indices clamped to prevent out-of-bounds panics if a display is disconnected while the app is running.

### 📦 Dependencies Added

- `tray-icon` — System tray icon with context menu

- `egui-phosphor 0.7` — Phosphor icon font for egui

### Known Limitations (unchanged from v1.0.0)

- Exclusive-fullscreen games are not supported (force-primary reserved for future).

- Live window list requires manual Refresh click.

---

**Initial release.**

### What's included

- **Profile-Based Launching** — Save EXE + monitor pairs as named profiles. Hit Launch and the app opens on the right screen, every time.

- **Window Process Tracking** — For launchers that spawn a different child process (e.g. a game client), specify the child EXE name so DisplayWarp waits for the right window.

- **Live Window Mover** — Move any already-running window to any monitor in a single click. No flicker, no restart needed.

- **Visual Monitor Preview** — Scaled layout diagram shows all connected displays. Highlighted green = currently targeted monitor, updates live while editing profiles.

- **Persistent Profiles** — Profiles saved to `monitor_config.json` next to the exe, survive restarts, human-readable and hand-editable.

### Notes

- Windows 10 / 11 only (Win32 API).

- No installer — single portable `.exe`.

- No admin rights required for normal window management.

### Known limitations

- Exclusive-fullscreen games that lock their window to the primary monitor are not supported in this release (force-primary switching is reserved for a future version).

- The live window list requires a manual Refresh click — it does not auto-update.
