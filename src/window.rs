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
    BringWindowToTop, EnumWindows, GWL_EXSTYLE, GetWindowLongW, GetWindowRect,
    GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId, HWND_TOP, IsWindowVisible,
    SW_MAXIMIZE, SW_RESTORE, SWP_FRAMECHANGED, SWP_SHOWWINDOW, SetForegroundWindow, SetWindowPos,
    ShowWindow, WS_EX_TOOLWINDOW,
};

// ─── Public types ─────────────────────────────────────────────────────────────

/// A visible top-level window that can be moved by the user.
#[derive(Clone)]
pub struct ProcessEntry {
    pub hwnd: HWND,
    #[allow(dead_code)]
    pub pid: u32,
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

fn find_window_by_process_name(target_lowercase: &str) -> Option<HWND> {
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
/// For live (already-running) windows. No retry loop, no watcher, no SW_MAXIMIZE/RESTORE.
pub fn move_window_once(hwnd: HWND, target_rect: RECT) {
    let w = target_rect.right - target_rect.left;
    let h = target_rect.bottom - target_rect.top;
    unsafe {
        // SetWindowPos alone can reposition any window including maximized ones.
        // Do NOT call SW_RESTORE first — it snaps a maximized window back to its
        // pre-maximize position (on the old monitor) before the new SetWindowPos
        // fires, causing a visible double-jump on every move.
        let _ = SetWindowPos(
            hwnd,
            HWND_TOP,
            target_rect.left,
            target_rect.top,
            w,
            h,
            SWP_SHOWWINDOW | SWP_FRAMECHANGED,
        );
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
            label: format!("{exe_name} — {title}"),
        });

        TRUE
    }
}
