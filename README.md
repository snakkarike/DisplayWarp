![DisplayWarp](https://raw.githubusercontent.com/snakkarike/DisplayWarp/main/assets/DisplayWarpLogo.svg)

#### Launch any application on any monitor, every time. Move live windows instantly.

##### Windows is notoriously bad at remembering which monitor you want your apps to open on. DisplayWarp is a lightweight system tray utility that fixes this "primary monitor monopoly" by intercepting app launches and forcing them onto the display of your choice.

[![Release](https://img.shields.io/github/v/release/snakkarike/DisplayWarp)](https://github.com/snakkarike/DisplayWarp/releases)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Google Gemini](https://img.shields.io/badge/Google%20Gemini-886FBF?logo=googlegemini&logoColor=fff)](#)
![Rust](https://img.shields.io/badge/Rust-000000?style=flat&logo=rust&logoColor=white)

[Quick Start](#quick-start) · [Building from Source](#building-from-source)

---

## Features

- **Profile-Based Launching** — Save a per-app rule: pick an EXE and a target monitor, click Launch. DisplayWarp opens the app and moves its window to the right screen automatically.
- **Live Window Mover** — See all running windows in a dropdown. Pick one and a target monitor, click Move — done instantly, no flicker.
- **Visual Monitor Preview** — A scaled layout preview shows all your connected displays. The selected target is highlighted green as you edit.
- **Window Process Tracking** — For launchers that spawn a child process (e.g. Battle.net → game), specify the child EXE name and DisplayWarp will wait for that window instead.
- **Persistent Profiles** — Profiles are saved to `monitor_config.json` next to the exe and survive restarts.
- **Zero Dependencies at Runtime** — Ships as a single `.exe`. No runtimes, no installers, no admin rights required for normal use.

---

## Quick Start

### Create a Launch Profile

1. Click **📁 Select EXE** and pick the application executable.
2. Choose the **target monitor** from the dropdown (the preview highlights it green).
3. _(Optional)_ If the EXE is a launcher that opens a different process (e.g. a game client), fill in the **Window process** field with the child process name, e.g. `Diablo IV.exe`.
4. Click **💾 Save Profile**.
5. Hit **▶ Launch** whenever you want to open the app on that monitor.

### Move a Running Window

1. In the **🖥 Move Live Window** section, click **🔄 Refresh** to list all open windows.
2. Pick the window from the dropdown.
3. Pick the target monitor.
4. Click **▶ Move**.

---

## How It Works

When a profile is launched, DisplayWarp:

1. Spawns the configured executable.
2. Polls for the window to appear (by PID, or by child process name if specified).
3. Repositions and focuses the window on the target monitor.

For the live mover, it issues a direct `SetWindowPos` call — no aggressive retries, no flickering, single-shot.

---

## Building from Source

**Requirements:** Rust stable toolchain (MSVC target), Windows 10/11.

```sh
git clone https://github.com/your-username/DisplayWarp
cd DisplayWarp
cargo build --release
```

The binary will be at `target/release/display_warp.exe`. Copy it anywhere you like.

---

## Configuration

Profiles are stored as plain JSON in `monitor_config.json` next to the executable:

```json
{ "profiles": [{ "name": "firefox.exe", "exe_path": "C:\\Program Files\\Mozilla Firefox\\firefox.exe", "target_monitor_name": "\\\\.\\DISPLAY2", "target_monitor_rect": { "left": 1920, "top": 0, "right": 3840, "bottom": 1080 }, "window_process_name": null, "force_primary": false }] }
```

You can edit this file by hand — changes take effect on next launch.

---

## Tech Stack

|               |                                                                                                           |
| ------------- | --------------------------------------------------------------------------------------------------------- |
| UI            | [egui](https://github.com/emilk/egui) / [eframe](https://github.com/emilk/egui/tree/master/crates/eframe) |
| Win32         | [windows-rs](https://github.com/microsoft/windows-rs)                                                     |
| Serialization | [serde](https://serde.rs) + serde_json                                                                    |
| File dialogs  | [rfd](https://github.com/PolyMeilex/rfd)                                                                  |

---

## License

MIT
