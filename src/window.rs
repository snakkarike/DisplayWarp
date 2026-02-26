use std::ptr;
use windows::Win32::Foundation::{BOOL, FALSE, HWND, LPARAM, POINT, RECT, TRUE};
use windows::Win32::Graphics::Gdi::{
    HMONITOR, MONITOR_DEFAULTTONEAREST, MonitorFromPoint, MonitorFromWindow,
};
use windows::Win32::System::Threading::{
    OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SYNCHRONIZE,
    QueryFullProcessImageNameW, WaitForSingleObject,
};
use windows::Win32::UI::WindowsAndMessaging::{
    BringWindowToTop, EnumWindows, GWL_EXSTYLE, GetWindowLongW, GetWindowPlacement, GetWindowRect,
    GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId, HWND_TOP, IsWindow,
    IsWindowVisible, SW_MAXIMIZE, SW_RESTORE, SW_SHOWMAXIMIZED, SWP_FRAMECHANGED, SWP_SHOWWINDOW,
    SetForegroundWindow, SetWindowPlacement, SetWindowPos, ShowWindow, WINDOWPLACEMENT,
    WS_EX_TOOLWINDOW,
};

// ─── Public types ─────────────────────────────────────────────────────────────

/// A visible top-level window that can be moved by the user.
#[derive(Clone)]
pub struct ProcessEntry {
    pub hwnd: HWND,
    #[allow(dead_code)]
    pub pid: u32,
    /// Full path to the executable, if it could be retrieved
    pub exe_path: Option<std::path::PathBuf>,
    /// Display label: "ExeName.exe — Window Title"
    pub label: String,
}

unsafe impl Send for ProcessEntry {}
unsafe impl Sync for ProcessEntry {}

// ─── Internal helper structs ─────────────────────────────────────────────────

struct WindowCandidate {
    hwnd: HWND,
    score: i32,
}

struct FindWindowByNameData {
    target_name: String, // lowercase
    candidates: Vec<WindowCandidate>,
}

struct FindWindowData {
    pid: u32,
    hwnd: HWND,
}

// ─── Public API ──────────────────────────────────────────────────────────────

/// Poll for a visible, top-level window owned by `pid`. Waits up to `timeout_ms`.
pub fn wait_for_window(pid: u32, timeout_ms: u64) -> Option<HWND> {
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(timeout_ms);
    loop {
        if let Some(h) = find_main_window_by_pid(pid) {
            return Some(h);
        }
        if std::time::Instant::now() >= deadline {
            return None;
        }
        std::thread::sleep(std::time::Duration::from_millis(300));
    }
}

/// Poll for a visible window whose owning process exe name matches `process_name` (case-insensitive).
/// Useful when a launcher spawns a different child process (e.g. Battle.net → actual game).
pub fn wait_for_window_by_name(process_name: &str, timeout_ms: u64) -> Option<HWND> {
    let target = process_name.to_lowercase();
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(timeout_ms);
    loop {
        if let Some(h) = find_window_by_process_name(&target) {
            return Some(h);
        }
        if std::time::Instant::now() >= deadline {
            return None;
        }
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
}

pub fn find_window_by_process_name(target_lowercase: &str) -> Option<HWND> {
    let mut data = FindWindowByNameData {
        target_name: target_lowercase.to_string(),
        candidates: Vec::new(),
    };
    unsafe {
        let _ = EnumWindows(
            Some(enum_window_by_name_callback),
            LPARAM(&mut data as *mut _ as isize),
        );
    }
    // Pick highest-scoring candidate
    data.candidates
        .into_iter()
        .max_by_key(|c| c.score)
        .map(|c| c.hwnd)
}

unsafe extern "system" fn enum_window_by_name_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
    unsafe {
        if !IsWindowVisible(hwnd).as_bool() {
            return TRUE;
        }
        let data = &mut *(lparam.0 as *mut FindWindowByNameData);
        let mut pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if pid == 0 {
            return TRUE;
        }
        if let Ok(hproc) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) {
            let mut buf = [0u16; 512];
            let mut len = buf.len() as u32;
            if QueryFullProcessImageNameW(
                hproc,
                PROCESS_NAME_WIN32,
                windows::core::PWSTR(buf.as_mut_ptr()),
                &mut len,
            )
            .is_ok()
            {
                let full_path = String::from_utf16_lossy(&buf[..len as usize]).to_lowercase();
                let exe_name = full_path.split(['/', '\\']).last().unwrap_or("");
                if exe_name == data.target_name {
                    // Score this candidate:
                    let mut score: i32 = 0;
                    // Prefer windows with a title (loader windows often have no title)
                    let title_len = GetWindowTextLengthW(hwnd);
                    if title_len > 0 {
                        score += 20;
                    }
                    // Penalise tool windows (overlays, toasts, etc)
                    let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;
                    if ex_style & WS_EX_TOOLWINDOW.0 != 0 {
                        score -= 50;
                    }
                    // Prefer larger windows (game window will be biggest)
                    let mut wr = RECT::default();
                    if GetWindowRect(hwnd, &mut wr).is_ok() {
                        let area = ((wr.right - wr.left) * (wr.bottom - wr.top)).max(0);
                        score += (area / 10_000).min(100); // cap at +100
                    }
                    data.candidates.push(WindowCandidate { hwnd, score });
                }
            }
            let _ = windows::Win32::Foundation::CloseHandle(hproc);
        }
        TRUE // always keep enumerating to collect all candidates
    }
}

fn find_main_window_by_pid(pid: u32) -> Option<HWND> {
    let mut data = FindWindowData {
        pid,
        hwnd: HWND(ptr::null_mut()),
    };
    unsafe {
        let _ = EnumWindows(
            Some(enum_window_callback),
            LPARAM(&mut data as *mut _ as isize),
        );
    }
    if data.hwnd.0.is_null() {
        None
    } else {
        Some(data.hwnd)
    }
}

unsafe extern "system" fn enum_window_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
    unsafe {
        let data = &mut *(lparam.0 as *mut FindWindowData);
        let mut window_pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut window_pid));
        if window_pid == data.pid && IsWindowVisible(hwnd).as_bool() {
            data.hwnd = hwnd;
            FALSE // Stop enumeration — we have our window
        } else {
            TRUE
        }
    }
}

/// Resolve the HMONITOR for the centre of a rect.
fn monitor_for_rect(rect: RECT) -> HMONITOR {
    let w = rect.right - rect.left;
    let h = rect.bottom - rect.top;
    unsafe {
        MonitorFromPoint(
            POINT {
                x: rect.left + w / 2,
                y: rect.top + h / 2,
            },
            MONITOR_DEFAULTTONEAREST,
        )
    }
}

/// Move a window to the target monitor rect in a single, non-flickering operation.
/// Uses SetWindowPlacement to update Windows' internal monitor tracking (so
/// minimize/maximize stays on the right screen) AND SetWindowPos for the
/// immediate visual move.
pub fn move_window_once(hwnd: HWND, target_rect: RECT) {
    let w = target_rect.right - target_rect.left;
    let h = target_rect.bottom - target_rect.top;
    unsafe {
        // Bail if the window handle is no longer valid (e.g. app closed,
        // or display layout change destroyed and recreated windows).
        if !IsWindow(hwnd).as_bool() {
            return;
        }

        // 1. Read current placement to detect if the window is maximized.
        let mut placement = WINDOWPLACEMENT {
            length: std::mem::size_of::<WINDOWPLACEMENT>() as u32,
            ..Default::default()
        };
        let _ = GetWindowPlacement(hwnd, &mut placement);
        let was_maximized = placement.showCmd == SW_MAXIMIZE.0 as u32
            || placement.showCmd == SW_SHOWMAXIMIZED.0 as u32;

        // 2. Update the "normal" (restored) position to be inside the target monitor.
        //    This is what Windows uses to decide which monitor to maximize on.
        let win_w = (w * 3) / 4;
        let win_h = (h * 3) / 4;
        let win_x = target_rect.left + (w - win_w) / 2;
        let win_y = target_rect.top + (h - win_h) / 2;
        placement.rcNormalPosition = RECT {
            left: win_x,
            top: win_y,
            right: win_x + win_w,
            bottom: win_y + win_h,
        };
        placement.showCmd = SW_RESTORE.0 as u32;
        let _ = SetWindowPlacement(hwnd, &placement);

        // 3. Visually move the window with SetWindowPos.
        let _ = SetWindowPos(
            hwnd,
            HWND_TOP,
            target_rect.left,
            target_rect.top,
            w,
            h,
            SWP_SHOWWINDOW | SWP_FRAMECHANGED,
        );

        // 4. If it was maximized, re-maximize on the new monitor.
        if was_maximized {
            let _ = ShowWindow(hwnd, SW_MAXIMIZE);
        }

        let _ = BringWindowToTop(hwnd);
        let _ = SetForegroundWindow(hwnd);
    }
}

/// Move a window to the target monitor rect, retrying until it sticks,
/// then watch it for 45 seconds and enforce position if the game moves it back.
pub fn move_to_monitor(hwnd: HWND, target_rect: RECT) {
    let w = target_rect.right - target_rect.left;
    let h = target_rect.bottom - target_rect.top;
    let target_mon = monitor_for_rect(target_rect);

    // ── Phase 1: aggressive initial placement (first ~6 s) ────────────────
    for attempt in 0..12u32 {
        unsafe {
            // Bail if window was destroyed (display change, app closed, etc.)
            if !IsWindow(hwnd).as_bool() {
                return;
            }
            if attempt > 0 {
                std::thread::sleep(std::time::Duration::from_millis(500));
            }
            let _ = BringWindowToTop(hwnd);
            let _ = SetForegroundWindow(hwnd);
            let _ = ShowWindow(hwnd, SW_RESTORE);
            std::thread::sleep(std::time::Duration::from_millis(60));
            let _ = SetWindowPos(
                hwnd,
                HWND_TOP,
                target_rect.left,
                target_rect.top,
                w,
                h,
                SWP_SHOWWINDOW | SWP_FRAMECHANGED,
            );
            let _ = ShowWindow(hwnd, SW_MAXIMIZE);
            std::thread::sleep(std::time::Duration::from_millis(100));
            if MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST) == target_mon {
                break;
            }
        }
    }

    // ── Phase 2: gentle keep-alive watcher for 45 s ─────────────────────
    // Only re-positions — does NOT call SW_MAXIMIZE — to avoid minimize/maximize flicker
    // on exclusive-fullscreen games that still ignore SetWindowPos.
    let watch_deadline = std::time::Instant::now() + std::time::Duration::from_secs(45);
    while std::time::Instant::now() < watch_deadline {
        std::thread::sleep(std::time::Duration::from_millis(1000));
        unsafe {
            // Stop watching if window was destroyed.
            if !IsWindow(hwnd).as_bool() {
                return;
            }
            if MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST) != target_mon {
                let _ = BringWindowToTop(hwnd);
                let _ = SetForegroundWindow(hwnd);
                let _ = ShowWindow(hwnd, SW_RESTORE);
                std::thread::sleep(std::time::Duration::from_millis(60));
                let _ = SetWindowPos(
                    hwnd,
                    HWND_TOP,
                    target_rect.left,
                    target_rect.top,
                    w,
                    h,
                    SWP_SHOWWINDOW | SWP_FRAMECHANGED,
                );
                // No SW_MAXIMIZE here — avoids flicker on exclusive-fullscreen games
            }
        }
    }
}

/// Block until a process (by PID) exits, or forever if it cannot be opened.
#[allow(dead_code)]
pub fn wait_for_pid_exit(pid: u32) {
    unsafe {
        if let Ok(hproc) = OpenProcess(PROCESS_SYNCHRONIZE, false, pid) {
            let _ = WaitForSingleObject(hproc, u32::MAX);
            let _ = windows::Win32::Foundation::CloseHandle(hproc);
        }
    }
}

// ─── Live process enumeration ────────────────────────────────────────────────

/// Return all visible top-level windows (with a title, not tool-windows) as
/// `ProcessEntry` items that the UI can present for the live-process mover.
pub fn list_visible_windows() -> Vec<ProcessEntry> {
    let mut entries: Vec<ProcessEntry> = Vec::new();
    unsafe {
        let _ = EnumWindows(
            Some(enum_visible_windows_callback),
            LPARAM(&mut entries as *mut _ as isize),
        );
    }
    // Sort by label for a stable, readable list
    entries.sort_by(|a, b| a.label.cmp(&b.label));
    entries
}

unsafe extern "system" fn enum_visible_windows_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
    unsafe {
        if !IsWindowVisible(hwnd).as_bool() {
            return TRUE;
        }
        // Skip tool windows
        let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;
        if ex_style & WS_EX_TOOLWINDOW.0 != 0 {
            return TRUE;
        }
        // Must have a title
        let title_len = GetWindowTextLengthW(hwnd);
        if title_len <= 0 {
            return TRUE;
        }
        let mut title_buf = vec![0u16; (title_len + 1) as usize];
        GetWindowTextW(hwnd, &mut title_buf);
        let title = String::from_utf16_lossy(&title_buf)
            .trim_matches('\0')
            .to_string();

        let mut pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));

        let mut exe_path = None;
        let exe_name = if let Ok(hproc) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid)
        {
            let mut buf = [0u16; 512];
            let mut len = buf.len() as u32;
            let name = if QueryFullProcessImageNameW(
                hproc,
                PROCESS_NAME_WIN32,
                windows::core::PWSTR(buf.as_mut_ptr()),
                &mut len,
            )
            .is_ok()
            {
                let full = String::from_utf16_lossy(&buf[..len as usize]);
                // Remove trailing nulls if any
                let full = full.trim_matches('\0');
                if !full.is_empty() {
                    exe_path = Some(std::path::PathBuf::from(full));
                }
                full.split(['/', '\\']).last().unwrap_or("").to_string()
            } else {
                format!("PID {pid}")
            };
            let _ = windows::Win32::Foundation::CloseHandle(hproc);
            name
        } else {
            format!("PID {pid}")
        };

        let entries = &mut *(lparam.0 as *mut Vec<ProcessEntry>);
        entries.push(ProcessEntry {
            hwnd,
            pid,
            exe_path,
            label: format!("{exe_name} — {title}"),
        });

        TRUE
    }
}
