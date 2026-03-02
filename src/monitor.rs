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

use windows::Win32::Devices::Display::{
    DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME, DISPLAYCONFIG_MODE_INFO, DISPLAYCONFIG_PATH_INFO,
    DISPLAYCONFIG_TARGET_DEVICE_NAME, DisplayConfigGetDeviceInfo, GetDisplayConfigBufferSizes,
    QDC_ONLY_ACTIVE_PATHS, QueryDisplayConfig,
};

/// Enumerate all connected monitors and return their info.
pub fn get_all_monitors() -> Vec<MonitorInfo> {
    let mut monitors: Vec<MonitorInfo> = Vec::new();
    unsafe {
        let _ = EnumDisplayMonitors(
            Some(HDC(ptr::null_mut())),
            None,
            Some(enum_monitor_callback),
            LPARAM(&mut monitors as *mut _ as isize),
        );

        // Fetch physical hardware names using QueryDisplayConfig
        let mut path_count = 0;
        let mut mode_count = 0;
        if GetDisplayConfigBufferSizes(QDC_ONLY_ACTIVE_PATHS, &mut path_count, &mut mode_count)
            == windows::Win32::Foundation::WIN32_ERROR(0)
        {
            let mut paths = vec![DISPLAYCONFIG_PATH_INFO::default(); path_count as usize];
            let mut modes = vec![DISPLAYCONFIG_MODE_INFO::default(); mode_count as usize];

            if QueryDisplayConfig(
                QDC_ONLY_ACTIVE_PATHS,
                &mut path_count,
                paths.as_mut_ptr(),
                &mut mode_count,
                modes.as_mut_ptr(),
                None,
            ) == windows::Win32::Foundation::WIN32_ERROR(0)
            {
                for path in paths.iter().take(path_count as usize) {
                    let mut target_name = DISPLAYCONFIG_TARGET_DEVICE_NAME::default();
                    target_name.header.r#type = DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME;
                    target_name.header.size =
                        std::mem::size_of::<DISPLAYCONFIG_TARGET_DEVICE_NAME>() as u32;
                    target_name.header.adapterId = path.targetInfo.adapterId;
                    target_name.header.id = path.targetInfo.id;

                    if DisplayConfigGetDeviceInfo(&mut target_name.header) == 0 {
                        let friendly_name =
                            String::from_utf16_lossy(&target_name.monitorFriendlyDeviceName)
                                .trim_matches(char::from(0))
                                .to_string();

                        // Assign sequentially
                        for mon in &mut monitors {
                            if mon.hardware_name.is_none() && !friendly_name.is_empty() {
                                mon.hardware_name = Some(friendly_name.clone());
                                mon.target_id = Some(path.targetInfo.id);
                                break;
                            }
                        }
                    }
                }
            }
        }
    }
    monitors
}

use windows::Win32::Devices::Display::{
    DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME, DISPLAYCONFIG_SOURCE_DEVICE_NAME, QDC_ALL_PATHS,
};

pub fn get_all_display_targets() -> Vec<MonitorInfo> {
    let mut targets: Vec<MonitorInfo> = Vec::new();
    unsafe {
        let mut path_count = 0;
        let mut mode_count = 0;
        if GetDisplayConfigBufferSizes(QDC_ALL_PATHS, &mut path_count, &mut mode_count)
            != windows::Win32::Foundation::WIN32_ERROR(0)
        {
            return targets;
        }

        let mut paths = vec![DISPLAYCONFIG_PATH_INFO::default(); path_count as usize];
        let mut modes = vec![DISPLAYCONFIG_MODE_INFO::default(); mode_count as usize];

        if QueryDisplayConfig(
            QDC_ALL_PATHS,
            &mut path_count,
            paths.as_mut_ptr(),
            &mut mode_count,
            modes.as_mut_ptr(),
            None,
        ) != windows::Win32::Foundation::WIN32_ERROR(0)
        {
            return targets;
        }

        let mut seen_targets = std::collections::HashSet::new();

        for path in paths.iter().take(path_count as usize) {
            let target_id = path.targetInfo.id;
            // DisplayConfig paths can have multiple entries for the same target (e.g. inactive permutations).
            // We only want to list each physical monitor once.
            if !seen_targets.insert(target_id) {
                // If this path is active, update the existing entry to active.
                let is_active = (path.flags & 1) != 0; // DISPLAYCONFIG_PATH_ACTIVE
                if is_active {
                    if let Some(t) = targets.iter_mut().find(|m| m.target_id == Some(target_id)) {
                        t.is_active = true;
                    }
                }
                continue;
            }

            let mut target_name = DISPLAYCONFIG_TARGET_DEVICE_NAME::default();
            target_name.header.r#type = DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME;
            target_name.header.size =
                std::mem::size_of::<DISPLAYCONFIG_TARGET_DEVICE_NAME>() as u32;
            target_name.header.adapterId = path.targetInfo.adapterId;
            target_name.header.id = target_id;

            let mut friendly_name = String::new();
            if DisplayConfigGetDeviceInfo(&mut target_name.header) == 0 {
                friendly_name = String::from_utf16_lossy(&target_name.monitorFriendlyDeviceName)
                    .trim_matches(char::from(0))
                    .to_string();
            }

            // Also try to get the Source Device Name (e.g. \\.\DISPLAY1) if active
            let mut device_name = String::new();
            let is_active = (path.flags & 1) != 0; // DISPLAYCONFIG_PATH_ACTIVE
            if is_active {
                let mut source_name = DISPLAYCONFIG_SOURCE_DEVICE_NAME::default();
                source_name.header.r#type = DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME;
                source_name.header.size =
                    std::mem::size_of::<DISPLAYCONFIG_SOURCE_DEVICE_NAME>() as u32;
                source_name.header.adapterId = path.sourceInfo.adapterId;
                source_name.header.id = path.sourceInfo.id;

                if DisplayConfigGetDeviceInfo(&mut source_name.header) == 0 {
                    device_name = String::from_utf16_lossy(&source_name.viewGdiDeviceName)
                        .trim_matches(char::from(0))
                        .to_string();
                }
            }

            if !is_active && friendly_name.is_empty() {
                continue;
            }

            targets.push(MonitorInfo {
                rect: RECT::default(), // Not relevant for topology toggling
                device_name,
                hardware_name: if friendly_name.is_empty() {
                    None
                } else {
                    Some(friendly_name)
                },
                target_id: Some(target_id),
                is_active,
            });
        }
    }

    // Sort so active targets appear first
    targets.sort_by(|a, b| b.is_active.cmp(&a.is_active));
    targets
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
                hardware_name: None,
                target_id: None,
                is_active: true,
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

use windows::Win32::Devices::Display::{
    SDC_ALLOW_CHANGES, SDC_APPLY, SDC_SAVE_TO_DATABASE, SDC_USE_SUPPLIED_DISPLAY_CONFIG,
    SetDisplayConfig,
};

/// Set the state of a specific monitor by Target ID.
/// `state` can be "Extend", "Duplicate", or "Disconnect".
pub fn set_monitor_state(target_id: u32, state: &str) {
    // If state is "Duplicate", it expects the format "DuplicateWith:{other_target_id}"
    let mut other_target_id: Option<u32> = None;
    let final_state = if state.starts_with("DuplicateWith:") {
        if let Ok(tid) = state.trim_start_matches("DuplicateWith:").parse::<u32>() {
            other_target_id = Some(tid);
        }
        "Duplicate"
    } else {
        state
    };

    unsafe {
        let mut path_count = 0;
        let mut mode_count = 0;
        if GetDisplayConfigBufferSizes(QDC_ALL_PATHS, &mut path_count, &mut mode_count)
            != windows::Win32::Foundation::WIN32_ERROR(0)
        {
            return;
        }

        let mut all_paths = vec![DISPLAYCONFIG_PATH_INFO::default(); path_count as usize];
        let mut all_modes = vec![DISPLAYCONFIG_MODE_INFO::default(); mode_count as usize];

        if QueryDisplayConfig(
            QDC_ALL_PATHS,
            &mut path_count,
            all_paths.as_mut_ptr(),
            &mut mode_count,
            all_modes.as_mut_ptr(),
            None,
        ) != windows::Win32::Foundation::WIN32_ERROR(0)
        {
            return;
        }

        // 1. Identify currently active paths
        let mut active_paths: Vec<DISPLAYCONFIG_PATH_INFO> = all_paths
            .iter()
            .take(path_count as usize)
            .filter(|p| (p.flags & 1) != 0)
            .cloned()
            .collect();

        let mut modified = false;

        match final_state {
            "Disconnect" => {
                let initial_len = active_paths.len();
                active_paths.retain(|p| p.targetInfo.id != target_id);
                if active_paths.len() != initial_len {
                    modified = true;
                }
            }
            "Extend" | "Reconnect" => {
                // Check if already active
                if !active_paths.iter().any(|p| p.targetInfo.id == target_id) {
                    // Find a suitable path in the full list
                    if let Some(mut path) = all_paths
                        .iter()
                        .take(path_count as usize)
                        .find(|p| p.targetInfo.id == target_id)
                        .cloned()
                    {
                        path.flags |= 1; // Mark active

                        // Ensure source ID doesn't conflict
                        let adapter_id = path.targetInfo.adapterId.LowPart;
                        let mut used_sources = std::collections::HashSet::new();
                        for ap in &active_paths {
                            if ap.targetInfo.adapterId.LowPart == adapter_id {
                                used_sources.insert(ap.sourceInfo.id);
                            }
                        }

                        if used_sources.contains(&path.sourceInfo.id) {
                            let mut new_source = 0;
                            while used_sources.contains(&new_source) {
                                new_source += 1;
                            }
                            path.sourceInfo.id = new_source;
                        }

                        active_paths.push(path);
                        modified = true;
                    }
                }
            }
            "Duplicate" => {
                // Find source of the other monitor
                let source_to_clone = if let Some(other_tid) = other_target_id {
                    active_paths
                        .iter()
                        .find(|p| p.targetInfo.id == other_tid)
                        .map(|p| (p.targetInfo.adapterId, p.sourceInfo.id))
                } else {
                    active_paths
                        .first()
                        .map(|p| (p.targetInfo.adapterId, p.sourceInfo.id))
                };

                if let Some((adapter_id, source_id)) = source_to_clone {
                    // Remove existing path for target_id if it exists
                    active_paths.retain(|p| p.targetInfo.id != target_id);
                    // Find it in the master list and clone the source
                    if let Some(mut path) = all_paths
                        .iter()
                        .take(path_count as usize)
                        .find(|p| p.targetInfo.id == target_id)
                        .cloned()
                    {
                        path.flags |= 1;
                        path.targetInfo.adapterId = adapter_id;
                        path.sourceInfo.id = source_id;
                        active_paths.push(path);
                        modified = true;
                    }
                }
            }
            _ => {}
        }

        if modified {
            // Reset mode indices for all paths to ensure a clean negotiation
            for p in &mut active_paths {
                p.sourceInfo.Anonymous.modeInfoIdx = 0xFFFFFFFF;
                p.targetInfo.Anonymous.modeInfoIdx = 0xFFFFFFFF;
            }

            let _ = SetDisplayConfig(
                Some(&active_paths),
                None, // We let Windows negotiate modes
                SDC_APPLY
                    | SDC_USE_SUPPLIED_DISPLAY_CONFIG
                    | SDC_SAVE_TO_DATABASE
                    | SDC_ALLOW_CHANGES,
            );
        }
    }
}

pub fn set_primary_monitor(target_id: u32) {
    unsafe {
        let mut path_count = 0;
        let mut mode_count = 0;
        if GetDisplayConfigBufferSizes(QDC_ONLY_ACTIVE_PATHS, &mut path_count, &mut mode_count)
            != windows::Win32::Foundation::WIN32_ERROR(0)
        {
            return;
        }

        let mut paths = vec![DISPLAYCONFIG_PATH_INFO::default(); path_count as usize];
        let mut modes = vec![DISPLAYCONFIG_MODE_INFO::default(); mode_count as usize];

        if QueryDisplayConfig(
            QDC_ONLY_ACTIVE_PATHS,
            &mut path_count,
            paths.as_mut_ptr(),
            &mut mode_count,
            modes.as_mut_ptr(),
            None,
        ) != windows::Win32::Foundation::WIN32_ERROR(0)
        {
            return;
        }

        // Adjust sizes to actual returned count
        paths.truncate(path_count as usize);

        // Find the index of the path we want to be primary
        if let Some(target_idx) = paths.iter().position(|p| p.targetInfo.id == target_id) {
            let target_source_idx = paths[target_idx].sourceInfo.Anonymous.modeInfoIdx;

            // If the structure provides mode details, we must recalculate origins
            if target_source_idx < mode_count {
                let offset_x = modes[target_source_idx as usize]
                    .Anonymous
                    .sourceMode
                    .position
                    .x;
                let offset_y = modes[target_source_idx as usize]
                    .Anonymous
                    .sourceMode
                    .position
                    .y;

                for mode in &mut modes {
                    if mode.infoType
                        == windows::Win32::Devices::Display::DISPLAYCONFIG_MODE_INFO_TYPE_SOURCE
                    {
                        mode.Anonymous.sourceMode.position.x -= offset_x;
                        mode.Anonymous.sourceMode.position.y -= offset_y;
                    }
                }
            }

            // Windows implicitly treats the path at index 0 as the primary display.
            // Move the target path to index 0.
            if target_idx != 0 {
                paths.swap(0, target_idx);
            }

            // Apply
            let _ = SetDisplayConfig(
                Some(&paths),
                Some(&modes),
                SDC_APPLY
                    | SDC_USE_SUPPLIED_DISPLAY_CONFIG
                    | SDC_SAVE_TO_DATABASE
                    | SDC_ALLOW_CHANGES,
            );
        }
    }
}
