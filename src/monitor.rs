use std::ptr;
use windows::Win32::Foundation::{LPARAM, RECT};
use windows::Win32::Graphics::Gdi::{
    CDS_GLOBAL, CDS_NORESET, CDS_SET_PRIMARY, CDS_TYPE, CDS_UPDATEREGISTRY,
    ChangeDisplaySettingsExW, DEVMODEW, DM_POSITION, ENUM_CURRENT_SETTINGS, EnumDisplayMonitors,
    EnumDisplaySettingsW, GetMonitorInfoW, HDC, HMONITOR, MONITORINFOEXW,
};
use windows::core::BOOL;
use windows::core::PCWSTR;

use crate::models::{MonitorInfo, SavedMonitorPos};

/// Enumerate all connected monitors and return their info.
pub fn get_all_monitors() -> Vec<MonitorInfo> {
    let mut monitors = Vec::new();
    unsafe {
        let _ = EnumDisplayMonitors(
            Some(HDC(ptr::null_mut())),
            None,
            Some(enum_monitor_callback),
            LPARAM(&mut monitors as *mut _ as isize),
        );
    }
    monitors
}

unsafe extern "system" fn enum_monitor_callback(
    hmon: HMONITOR,
    _: HDC,
    _: *mut RECT,
    lparam: LPARAM,
) -> BOOL {
    unsafe {
        let monitors = &mut *(lparam.0 as *mut Vec<MonitorInfo>);
        let mut info = MONITORINFOEXW::default();
        info.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;
        if GetMonitorInfoW(hmon, &mut info.monitorInfo).as_bool() {
            let device_name = String::from_utf16_lossy(&info.szDevice)
                .trim_matches(char::from(0))
                .to_string();
            monitors.push(MonitorInfo {
                rect: info.monitorInfo.rcMonitor,
                device_name,
            });
        }
        BOOL(1)
    }
}

/// Make `target_device_name` the primary monitor by shifting all monitor
/// coordinates so the target sits at (0, 0).
#[allow(dead_code)]
pub fn switch_primary_to(target_device_name: &str, monitors: &[MonitorInfo]) -> bool {
    let target = match monitors
        .iter()
        .find(|m| m.device_name == target_device_name)
    {
        Some(m) => m,
        None => return false,
    };
    let offset_x = target.rect.left;
    let offset_y = target.rect.top;

    unsafe {
        for mon in monitors {
            let name_u16: Vec<u16> = mon.device_name.encode_utf16().chain(Some(0)).collect();
            let mut dev_mode = std::mem::zeroed::<DEVMODEW>();
            dev_mode.dmSize = std::mem::size_of::<DEVMODEW>() as u16;
            if !EnumDisplaySettingsW(
                PCWSTR(name_u16.as_ptr()),
                ENUM_CURRENT_SETTINGS,
                &mut dev_mode,
            )
            .as_bool()
            {
                continue;
            }
            dev_mode.dmFields = DM_POSITION;
            dev_mode.Anonymous1.Anonymous2.dmPosition.x = mon.rect.left - offset_x;
            dev_mode.Anonymous1.Anonymous2.dmPosition.y = mon.rect.top - offset_y;

            let mut flags = CDS_UPDATEREGISTRY | CDS_NORESET | CDS_GLOBAL;
            if mon.device_name == target_device_name {
                flags |= CDS_SET_PRIMARY;
            }
            ChangeDisplaySettingsExW(
                PCWSTR(name_u16.as_ptr()),
                Some(&dev_mode),
                None,
                flags,
                None,
            );
        }
        // Flush all changes at once
        let _ = ChangeDisplaySettingsExW(PCWSTR(ptr::null()), None, None, CDS_TYPE(0), None);
    }
    true
}

/// Restore monitor positions from a saved snapshot.
#[allow(dead_code)]
pub fn restore_monitor_layout(snapshot: &[SavedMonitorPos]) {
    unsafe {
        for saved in snapshot {
            let name_u16: Vec<u16> = saved.device_name.encode_utf16().chain(Some(0)).collect();
            let mut dev_mode = std::mem::zeroed::<DEVMODEW>();
            dev_mode.dmSize = std::mem::size_of::<DEVMODEW>() as u16;
            if !EnumDisplaySettingsW(
                PCWSTR(name_u16.as_ptr()),
                ENUM_CURRENT_SETTINGS,
                &mut dev_mode,
            )
            .as_bool()
            {
                continue;
            }
            dev_mode.dmFields = DM_POSITION;
            dev_mode.Anonymous1.Anonymous2.dmPosition.x = saved.rect.left;
            dev_mode.Anonymous1.Anonymous2.dmPosition.y = saved.rect.top;
            // Mark as primary if it was primary (i.e., its top-left was (0,0))
            let mut flags = CDS_UPDATEREGISTRY | CDS_NORESET | CDS_GLOBAL;
            if saved.rect.left == 0 && saved.rect.top == 0 {
                flags |= CDS_SET_PRIMARY;
            }
            ChangeDisplaySettingsExW(
                PCWSTR(name_u16.as_ptr()),
                Some(&dev_mode),
                None,
                flags,
                None,
            );
        }
        let _ = ChangeDisplaySettingsExW(PCWSTR(ptr::null()), None, None, CDS_TYPE(0), None);
    }
}
