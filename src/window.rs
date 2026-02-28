use std::ptr;
use windows::Win32::Foundation::{HWND, LPARAM, RECT};
use windows::Win32::Graphics::Gdi::{
    HMONITOR, MONITOR_DEFAULTTONEAREST, MonitorFromRect, MonitorFromWindow,
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
use windows::core::BOOL;

use serde::{Deserialize, Serialize};

// ─── Public types ─────────────────────────────────────────────────────────────

#[derive(Clone, Serialize, Deserialize)]
pub struct ProcessEntry {
    pub hwnd: isize,
    #[allow(dead_code)]
    pub pid: u32,
    pub exe_path: Option<std::path::PathBuf>,
    pub label: String,
}

unsafe impl Send for ProcessEntry {}
unsafe impl Sync for ProcessEntry {}

/// Rich result returned by window-find functions so callers can log details.
pub struct WindowFound {
    pub hwnd: HWND,
    pub width: i32,
    pub height: i32,
    pub elapsed_ms: u64,
}

// ─── Internal helper structs ──────────────────────────────────────────────────

struct WindowCandidate {
    hwnd: HWND,
    score: i32,
    width: i32,
    height: i32,
}

struct FindWindowByNameData {
    target_name: String,
    candidates: Vec<WindowCandidate>,
}

struct FindWindowData {
    pid: u32,
    hwnd: HWND,
}

// ─── Public API ───────────────────────────────────────────────────────────────

/// Poll for a visible, top-level window owned by `pid`. Waits up to `timeout_ms`.
pub fn wait_for_window(pid: u32, timeout_ms: u64) -> Option<WindowFound> {
    let start = std::time::Instant::now();
    let deadline = start + std::time::Duration::from_millis(timeout_ms);
    loop {
        if let Some(hwnd) = find_main_window_by_pid(pid) {
            let mut wr = RECT::default();
            unsafe {
                let _ = GetWindowRect(hwnd, &mut wr);
            }
            return Some(WindowFound {
                hwnd,
                width: wr.right - wr.left,
                height: wr.bottom - wr.top,
                elapsed_ms: start.elapsed().as_millis() as u64,
            });
        }
        if std::time::Instant::now() >= deadline {
            return None;
        }
        std::thread::sleep(std::time::Duration::from_millis(300));
    }
}

/// Poll for a visible window whose owning process exe name matches `process_name`
/// (case-insensitive). Returns rich info about the found window.
pub fn wait_for_window_by_name(process_name: &str, timeout_ms: u64) -> Option<WindowFound> {
    let target = process_name.to_lowercase();
    let start = std::time::Instant::now();
    let deadline = start + std::time::Duration::from_millis(timeout_ms);

    loop {
        if let Some(found) = find_best_window_by_process_name(&target) {
            let elapsed_ms = start.elapsed().as_millis() as u64;
            return Some(WindowFound {
                elapsed_ms,
                ..found
            });
        }
        if std::time::Instant::now() >= deadline {
            return None;
        }
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
}

pub fn find_window_by_process_name(target_lowercase: &str) -> Option<HWND> {
    find_best_window_by_process_name(target_lowercase).map(|f| f.hwnd)
}

fn find_best_window_by_process_name(target_lowercase: &str) -> Option<WindowFound> {
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
    let best = data.candidates.into_iter().max_by_key(|c| c.score)?;
    if best.score > 0 {
        Some(WindowFound {
            hwnd: best.hwnd,
            width: best.width,
            height: best.height,
            elapsed_ms: 0,
        })
    } else {
        None
    }
}

unsafe extern "system" fn enum_window_by_name_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
    unsafe {
        if !IsWindowVisible(hwnd).as_bool() {
            return BOOL(1);
        }
        let data = &mut *(lparam.0 as *mut FindWindowByNameData);
        let mut pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if pid == 0 {
            return BOOL(1);
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
                let exe_name = full_path
                    .trim_matches('\0')
                    .split(['/', '\\'])
                    .last()
                    .unwrap_or("")
                    .to_string();

                if exe_name == data.target_name {
                    let mut score: i32 = 0;
                    let mut w = 0i32;
                    let mut h = 0i32;

                    let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;
                    if ex_style & WS_EX_TOOLWINDOW.0 != 0 {
                        score -= 50;
                    }

                    let mut wr = RECT::default();
                    if GetWindowRect(hwnd, &mut wr).is_ok() {
                        w = wr.right - wr.left;
                        h = wr.bottom - wr.top;
                        let area = (w * h).max(0);
                        if w < 480 || h < 270 {
                            score -= 100;
                        } else {
                            score += (area / 10_000).min(200);
                        }
                    }

                    let title_len = GetWindowTextLengthW(hwnd);
                    if title_len > 0 {
                        score += 20;
                    }

                    data.candidates.push(WindowCandidate {
                        hwnd,
                        score,
                        width: w,
                        height: h,
                    });
                }
            }
            let _ = windows::Win32::Foundation::CloseHandle(hproc);
        }
        BOOL(1)
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
            BOOL(0)
        } else {
            BOOL(1)
        }
    }
}

fn monitor_for_rect(rect: RECT) -> HMONITOR {
    unsafe { MonitorFromRect(&rect, MONITOR_DEFAULTTONEAREST) }
}

pub fn move_window_once(hwnd: HWND, target_rect: RECT) {
    let w = target_rect.right - target_rect.left;
    let h = target_rect.bottom - target_rect.top;
    unsafe {
        if !IsWindow(Some(hwnd)).as_bool() {
            return;
        }
        let mut placement = WINDOWPLACEMENT {
            length: std::mem::size_of::<WINDOWPLACEMENT>() as u32,
            ..Default::default()
        };
        let _ = GetWindowPlacement(hwnd, &mut placement);
        let was_maximized = placement.showCmd == SW_MAXIMIZE.0 as u32
            || placement.showCmd == SW_SHOWMAXIMIZED.0 as u32;

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

        let _ = SetWindowPos(
            hwnd,
            Some(HWND_TOP),
            target_rect.left,
            target_rect.top,
            w,
            h,
            SWP_SHOWWINDOW | SWP_FRAMECHANGED,
        );

        if was_maximized {
            let _ = ShowWindow(hwnd, SW_MAXIMIZE);
        }

        let _ = BringWindowToTop(hwnd);
        let _ = SetForegroundWindow(hwnd);
    }
}

/// Silently watch a window for `watch_secs` seconds and nudge it back if it
/// drifts off the target monitor. Call this in a background thread after the
/// initial move so it never blocks the caller.
pub fn watch_window_on_monitor(hwnd: HWND, target_rect: RECT, watch_secs: u64) {
    let w = target_rect.right - target_rect.left;
    let h = target_rect.bottom - target_rect.top;
    let target_mon = monitor_for_rect(target_rect);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(watch_secs);

    while std::time::Instant::now() < deadline {
        std::thread::sleep(std::time::Duration::from_millis(1000));
        unsafe {
            if !IsWindow(Some(hwnd)).as_bool() {
                return;
            }
            if MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST) != target_mon {
                let _ = BringWindowToTop(hwnd);
                let _ = SetForegroundWindow(hwnd);
                let _ = ShowWindow(hwnd, SW_RESTORE);
                std::thread::sleep(std::time::Duration::from_millis(60));
                let _ = SetWindowPos(
                    hwnd,
                    Some(HWND_TOP),
                    target_rect.left,
                    target_rect.top,
                    w,
                    h,
                    SWP_SHOWWINDOW | SWP_FRAMECHANGED,
                );
            }
        }
    }
}

#[allow(dead_code)]
pub fn move_to_monitor(hwnd: HWND, target_rect: RECT) {
    let w = target_rect.right - target_rect.left;
    let h = target_rect.bottom - target_rect.top;
    let target_mon = monitor_for_rect(target_rect);

    for attempt in 0..12u32 {
        unsafe {
            if !IsWindow(Some(hwnd)).as_bool() {
                return;
            }
            if attempt > 0 {
                std::thread::sleep(std::time::Duration::from_millis(500));
            }

            let mut placement = WINDOWPLACEMENT {
                length: std::mem::size_of::<WINDOWPLACEMENT>() as u32,
                ..Default::default()
            };
            let _ = GetWindowPlacement(hwnd, &mut placement);

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

            let _ = BringWindowToTop(hwnd);
            let _ = SetForegroundWindow(hwnd);

            let _ = SetWindowPos(
                hwnd,
                Some(HWND_TOP),
                target_rect.left,
                target_rect.top,
                w,
                h,
                SWP_SHOWWINDOW | SWP_FRAMECHANGED,
            );

            let _ = ShowWindow(hwnd, SW_MAXIMIZE);

            std::thread::sleep(std::time::Duration::from_millis(150));
            if MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST) == target_mon {
                break;
            }
        }
    }

    let watch_deadline = std::time::Instant::now() + std::time::Duration::from_secs(45);
    while std::time::Instant::now() < watch_deadline {
        std::thread::sleep(std::time::Duration::from_millis(1000));
        unsafe {
            if !IsWindow(Some(hwnd)).as_bool() {
                return;
            }
            if MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST) != target_mon {
                let _ = BringWindowToTop(hwnd);
                let _ = SetForegroundWindow(hwnd);
                let _ = ShowWindow(hwnd, SW_RESTORE);
                std::thread::sleep(std::time::Duration::from_millis(60));
                let _ = SetWindowPos(
                    hwnd,
                    Some(HWND_TOP),
                    target_rect.left,
                    target_rect.top,
                    w,
                    h,
                    SWP_SHOWWINDOW | SWP_FRAMECHANGED,
                );
            }
        }
    }
}

#[allow(dead_code)]
pub fn wait_for_pid_exit(pid: u32) {
    unsafe {
        if let Ok(hproc) = OpenProcess(PROCESS_SYNCHRONIZE, false, pid) {
            let _ = WaitForSingleObject(hproc, u32::MAX);
            let _ = windows::Win32::Foundation::CloseHandle(hproc);
        }
    }
}

pub fn list_visible_windows() -> Vec<ProcessEntry> {
    let mut entries: Vec<ProcessEntry> = Vec::new();
    unsafe {
        let _ = EnumWindows(
            Some(enum_visible_windows_callback),
            LPARAM(&mut entries as *mut _ as isize),
        );
    }
    entries.sort_by(|a, b| a.label.cmp(&b.label));
    entries
}

unsafe extern "system" fn enum_visible_windows_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
    unsafe {
        if !IsWindowVisible(hwnd).as_bool() {
            return BOOL(1);
        }
        let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;
        if ex_style & WS_EX_TOOLWINDOW.0 != 0 {
            return BOOL(1);
        }
        let title_len = GetWindowTextLengthW(hwnd);
        if title_len <= 0 {
            return BOOL(1);
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
            hwnd: hwnd.0 as isize,
            pid,
            exe_path,
            label: format!("{exe_name} — {title}"),
        });

        BOOL(1)
    }
}
